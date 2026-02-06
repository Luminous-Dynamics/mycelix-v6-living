//! WebSocket server implementation.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use cycle_engine::{CycleEngineBuilder, MetabolismCycleEngine};
use living_core::CyclePhase;

use crate::rpc::{RpcError, RpcRequest, RpcResponse};

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to for WebSocket
    pub bind_addr: SocketAddr,
    /// Address to bind to for health/metrics HTTP
    pub health_addr: Option<SocketAddr>,
    /// Broadcast channel capacity for events
    pub broadcast_capacity: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8888".parse().unwrap(),
            health_addr: Some("127.0.0.1:8889".parse().unwrap()),
            broadcast_capacity: 1024,
        }
    }
}

/// Server metrics for observability.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub uptime_seconds: u64,
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

/// Atomic metrics counters.
#[derive(Debug, Default)]
pub struct AtomicMetrics {
    pub active_connections: AtomicU64,
    pub total_connections: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
}

/// WebSocket RPC server for the Living Protocol.
pub struct WebSocketServer {
    config: ServerConfig,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    event_tx: broadcast::Sender<String>,
    metrics: Arc<AtomicMetrics>,
    start_time: Instant,
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
            metrics: Arc::new(AtomicMetrics::default()),
            start_time: Instant::now(),
        }
    }

    /// Create a server with a custom cycle engine.
    pub fn with_engine(config: ServerConfig, engine: MetabolismCycleEngine) -> Self {
        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            event_tx,
            metrics: Arc::new(AtomicMetrics::default()),
            start_time: Instant::now(),
        }
    }

    /// Get current server metrics.
    pub fn get_metrics(&self) -> ServerMetrics {
        ServerMetrics {
            active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
            total_connections: self.metrics.total_connections.load(Ordering::Relaxed),
            messages_received: self.metrics.messages_received.load(Ordering::Relaxed),
            messages_sent: self.metrics.messages_sent.load(Ordering::Relaxed),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Start the WebSocket server.
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("WebSocket server listening on {}", self.config.bind_addr);

        // Start health/metrics HTTP server if configured
        if let Some(health_addr) = self.config.health_addr {
            let metrics = Arc::clone(&self.metrics);
            let engine = Arc::clone(&self.engine);
            tokio::spawn(async move {
                if let Err(e) = Self::run_health_server(health_addr, metrics, engine).await {
                    error!("Health server error: {}", e);
                }
            });
            info!("Health/metrics server listening on {}", health_addr);
        }

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

        // Accept connections with graceful shutdown support
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            info!("New connection from {}", addr);

                            self.metrics.total_connections.fetch_add(1, Ordering::Relaxed);
                            self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);

                            let engine = Arc::clone(&self.engine);
                            let event_rx = self.event_tx.subscribe();
                            let metrics = Arc::clone(&self.metrics);

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection_with_metrics(
                                    stream, addr, engine, event_rx, metrics
                                ).await {
                                    error!("Connection error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutdown signal received, gracefully shutting down...");
                    break;
                }
            }
        }

        // Graceful shutdown: wait for connections to drain
        info!("Waiting for active connections to close...");
        let mut wait_count = 0;
        while self.metrics.active_connections.load(Ordering::Relaxed) > 0 && wait_count < 30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            wait_count += 1;
            info!(
                "Active connections: {}",
                self.metrics.active_connections.load(Ordering::Relaxed)
            );
        }

        info!("Server shutdown complete");
        Ok(())
    }

    /// Run the health/metrics HTTP server.
    async fn run_health_server(
        addr: SocketAddr,
        metrics: Arc<AtomicMetrics>,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
    ) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (mut stream, _) = listener.accept().await?;
            let metrics = Arc::clone(&metrics);
            let engine = Arc::clone(&engine);

            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                if stream.read(&mut buf).await.is_err() {
                    return;
                }

                let request = String::from_utf8_lossy(&buf);
                let (status, body) = if request.starts_with("GET /health") {
                    ("200 OK", r#"{"status":"healthy"}"#.to_string())
                } else if request.starts_with("GET /metrics") {
                    let m = ServerMetrics {
                        active_connections: metrics.active_connections.load(Ordering::Relaxed),
                        total_connections: metrics.total_connections.load(Ordering::Relaxed),
                        messages_received: metrics.messages_received.load(Ordering::Relaxed),
                        messages_sent: metrics.messages_sent.load(Ordering::Relaxed),
                        uptime_seconds: 0, // Can't easily get this here
                    };
                    ("200 OK", serde_json::to_string(&m).unwrap_or_default())
                } else if request.starts_with("GET /state") {
                    let engine = engine.read().await;
                    let state = CycleStateResponse {
                        cycle_number: engine.cycle_number(),
                        current_phase: engine.current_phase(),
                        phase_started: engine.phase_started().to_rfc3339(),
                        cycle_started: engine.cycle_started().to_rfc3339(),
                        phase_day: engine.phase_day(),
                    };
                    ("200 OK", serde_json::to_string(&state).unwrap_or_default())
                } else {
                    ("404 Not Found", r#"{"error":"not found"}"#.to_string())
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    body.len(),
                    body
                );

                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    }

    /// Handle connection with metrics tracking.
    async fn handle_connection_with_metrics(
        stream: TcpStream,
        addr: SocketAddr,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        event_rx: broadcast::Receiver<String>,
        metrics: Arc<AtomicMetrics>,
    ) -> anyhow::Result<()> {
        let result = Self::handle_connection(stream, addr, engine, event_rx).await;
        metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
        result
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
                    Some(phase_str) => match phase_str {
                        "Shadow" => CyclePhase::Shadow,
                        "Composting" => CyclePhase::Composting,
                        "Liminal" => CyclePhase::Liminal,
                        "NegativeCapability" => CyclePhase::NegativeCapability,
                        "Eros" => CyclePhase::Eros,
                        "CoCreation" => CyclePhase::CoCreation,
                        "Beauty" => CyclePhase::Beauty,
                        "EmergentPersonhood" => CyclePhase::EmergentPersonhood,
                        "Kenosis" => CyclePhase::Kenosis,
                        _ => {
                            return RpcResponse::error(
                                request.id.clone(),
                                RpcError::invalid_params(&format!("Unknown phase: {}", phase_str)),
                            )
                        }
                    },
                    None => {
                        return RpcResponse::error(
                            request.id.clone(),
                            RpcError::invalid_params("Missing 'phase' parameter"),
                        )
                    }
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
                    None => {
                        return RpcResponse::error(
                            request.id.clone(),
                            RpcError::invalid_params("Missing 'operation' parameter"),
                        )
                    }
                };

                let engine = engine.read().await;
                let permitted = engine.is_operation_permitted(operation);
                RpcResponse::success(request.id.clone(), permitted)
            }

            _ => RpcResponse::error(
                request.id.clone(),
                RpcError::method_not_found(&request.method),
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
