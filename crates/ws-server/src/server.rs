//! WebSocket server implementation.

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use cycle_engine::{MetabolismCycleEngine, CycleEngineBuilder};
use living_core::CyclePhase;

use crate::rpc::{RpcError, RpcRequest, RpcResponse};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
    /// Broadcast channel capacity for events
    pub broadcast_capacity: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8888".parse().unwrap(),
            broadcast_capacity: 1024,
        }
    }
}

/// Cycle state for RPC responses (serializable version).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CycleStateResponse {
    pub cycle_number: u64,
    pub current_phase: CyclePhase,
    pub phase_started: String,
    pub cycle_started: String,
    pub phase_day: u32,
}


/// Phase transition for history responses.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseTransitionResponse {
    pub from: CyclePhase,
    pub to: CyclePhase,
    pub cycle_number: u64,
    pub transitioned_at: String,
}

/// WebSocket RPC server for the Living Protocol.
pub struct WebSocketServer {
    config: ServerConfig,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    event_tx: broadcast::Sender<String>,
}

impl WebSocketServer {
    /// Create a new server with default cycle engine.
    pub fn new(config: ServerConfig) -> Self {
        let engine = CycleEngineBuilder::new().build();
        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            event_tx,
        }
    }

    /// Create a server with a custom cycle engine.
    pub fn with_engine(config: ServerConfig, engine: MetabolismCycleEngine) -> Self {
        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            event_tx,
        }
    }

    /// Start the WebSocket server.
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("WebSocket server listening on {}", self.config.bind_addr);

        // Start the cycle engine
        {
            let mut engine = self.engine.write().await;
            engine.start()?;
            info!("Cycle engine started");
        }

        // Spawn tick loop
        let engine_clone = Arc::clone(&self.engine);
        let event_tx_clone = self.event_tx.clone();
        tokio::spawn(async move {
            Self::tick_loop(engine_clone, event_tx_clone).await;
        });

        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            info!("New connection from {}", addr);

            let engine = Arc::clone(&self.engine);
            let event_rx = self.event_tx.subscribe();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, engine, event_rx).await {
                    error!("Connection error from {}: {}", addr, e);
                }
            });
        }

        Ok(())
    }

    /// Tick loop to drive the cycle engine.
    async fn tick_loop(
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        event_tx: broadcast::Sender<String>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        loop {
            interval.tick().await;

            let events = {
                let mut engine = engine.write().await;
                match engine.tick() {
                    Ok(events) => events,
                    Err(e) => {
                        warn!("Tick error: {}", e);
                        continue;
                    }
                }
            };

            // Broadcast events to all connected clients
            for event in events {
                if let Ok(json) = serde_json::to_string(&event) {
                    let _ = event_tx.send(json);
                }
            }
        }
    }

    /// Handle a single WebSocket connection.
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        mut event_rx: broadcast::Receiver<String>,
    ) -> anyhow::Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                // Handle incoming messages from client
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!("Received from {}: {}", addr, text);

                            // Try to parse as RPC request
                            match serde_json::from_str::<RpcRequest>(&text) {
                                Ok(request) => {
                                    let response = Self::handle_request(&request, &engine).await;
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        write.send(Message::Text(json)).await?;
                                    }
                                }
                                Err(e) => {
                                    // Check if it's a ping
                                    if let Ok(obj) = serde_json::from_str::<Value>(&text) {
                                        if obj.get("type").and_then(|v| v.as_str()) == Some("ping") {
                                            let pong = serde_json::json!({"type": "pong"});
                                            write.send(Message::Text(pong.to_string())).await?;
                                            continue;
                                        }
                                    }
                                    warn!("Failed to parse request from {}: {}", addr, e);
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Client {} disconnected", addr);
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error from {}: {}", addr, e);
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }

                // Forward events to client
                event = event_rx.recv() => {
                    match event {
                        Ok(json) => {
                            if let Err(e) = write.send(Message::Text(json)).await {
                                error!("Failed to send event to {}: {}", addr, e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Client {} lagged behind by {} events", addr, n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle an RPC request.
    async fn handle_request(
        request: &RpcRequest,
        engine: &Arc<RwLock<MetabolismCycleEngine>>,
    ) -> RpcResponse {
        match request.method.as_str() {
            "getCycleState" => {
                let engine = engine.read().await;
                let state = CycleStateResponse {
                    cycle_number: engine.cycle_number(),
                    current_phase: engine.current_phase(),
                    phase_started: engine.phase_started().to_rfc3339(),
                    cycle_started: engine.cycle_started().to_rfc3339(),
                    phase_day: engine.phase_day(),
                };
                RpcResponse::success(request.id.clone(), state)
            }

            "getTransitionHistory" => {
                let engine = engine.read().await;
                let history: Vec<PhaseTransitionResponse> = engine
                    .transition_history()
                    .iter()
                    .map(|t| PhaseTransitionResponse {
                        from: t.from,
                        to: t.to,
                        cycle_number: t.cycle_number,
                        transitioned_at: t.transitioned_at.to_rfc3339(),
                    })
                    .collect();
                RpcResponse::success(request.id.clone(), history)
            }

            "getPhaseMetrics" => {
                // Parse phase from params
                let phase = match request.params.get("phase").and_then(|v| v.as_str()) {
                    Some(phase_str) => {
                        match phase_str {
                            "Shadow" => CyclePhase::Shadow,
                            "Composting" => CyclePhase::Composting,
                            "Liminal" => CyclePhase::Liminal,
                            "NegativeCapability" => CyclePhase::NegativeCapability,
                            "Eros" => CyclePhase::Eros,
                            "CoCreation" => CyclePhase::CoCreation,
                            "Beauty" => CyclePhase::Beauty,
                            "EmergentPersonhood" => CyclePhase::EmergentPersonhood,
                            "Kenosis" => CyclePhase::Kenosis,
                            _ => return RpcResponse::error(
                                request.id.clone(),
                                RpcError::invalid_params(&format!("Unknown phase: {}", phase_str))
                            ),
                        }
                    }
                    None => return RpcResponse::error(
                        request.id.clone(),
                        RpcError::invalid_params("Missing 'phase' parameter")
                    ),
                };

                let engine = engine.read().await;
                let metrics = engine.phase_metrics(phase);
                RpcResponse::success(request.id.clone(), metrics)
            }

            "getCurrentPhase" => {
                let engine = engine.read().await;
                RpcResponse::success(request.id.clone(), engine.current_phase())
            }

            "getCycleNumber" => {
                let engine = engine.read().await;
                RpcResponse::success(request.id.clone(), engine.cycle_number())
            }

            "isOperationPermitted" => {
                let operation = match request.params.get("operation").and_then(|v| v.as_str()) {
                    Some(op) => op,
                    None => return RpcResponse::error(
                        request.id.clone(),
                        RpcError::invalid_params("Missing 'operation' parameter")
                    ),
                };

                let engine = engine.read().await;
                let permitted = engine.is_operation_permitted(operation);
                RpcResponse::success(request.id.clone(), permitted)
            }

            _ => RpcResponse::error(
                request.id.clone(),
                RpcError::method_not_found(&request.method)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr.port(), 8888);
        assert_eq!(config.broadcast_capacity, 1024);
    }
}
