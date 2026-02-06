//! WebSocket server implementation with OpenTelemetry tracing.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, info_span, warn, Instrument};

use cycle_engine::{CycleEngineBuilder, MetabolismCycleEngine};
use living_core::{CyclePhase, LivingProtocolEvent};

use crate::auth::{AuthConfig, AuthCredentials, Authenticator, SharedAuthenticator};
use crate::rate_limit::{RateLimitConfig, RateLimiter, SharedRateLimiter};
use crate::rpc::{RpcError, RpcRequest, RpcResponse};
use crate::telemetry::trace_id_field;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to for WebSocket
    pub bind_addr: SocketAddr,
    /// Address to bind to for health/metrics HTTP
    pub health_addr: Option<SocketAddr>,
    /// Broadcast channel capacity for events
    pub broadcast_capacity: usize,
    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8888".parse().unwrap(),
            health_addr: Some("127.0.0.1:8889".parse().unwrap()),
            broadcast_capacity: 1024,
            rate_limit: RateLimitConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Create a new server config with custom security settings.
    pub fn with_security(
        bind_addr: SocketAddr,
        health_addr: Option<SocketAddr>,
        rate_limit: RateLimitConfig,
        auth: AuthConfig,
    ) -> Self {
        Self {
            bind_addr,
            health_addr,
            broadcast_capacity: 1024,
            rate_limit,
            auth,
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
    /// Rate limiter for connection and request throttling
    rate_limiter: SharedRateLimiter,
    /// Authenticator for validating credentials
    authenticator: SharedAuthenticator,
}

impl WebSocketServer {
    /// Create a new server with default cycle engine.
    pub fn new(config: ServerConfig) -> Self {
        let engine = CycleEngineBuilder::new().build();
        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let authenticator = Arc::new(Authenticator::new(config.auth.clone()));

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            event_tx,
            metrics: Arc::new(AtomicMetrics::default()),
            start_time: Instant::now(),
            rate_limiter,
            authenticator,
        }
    }

    /// Create a server with a custom cycle engine.
    pub fn with_engine(config: ServerConfig, engine: MetabolismCycleEngine) -> Self {
        let (event_tx, _) = broadcast::channel(config.broadcast_capacity);
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let authenticator = Arc::new(Authenticator::new(config.auth.clone()));

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            event_tx,
            metrics: Arc::new(AtomicMetrics::default()),
            start_time: Instant::now(),
            rate_limiter,
            authenticator,
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

    /// Get a reference to the engine for REST API integration.
    pub fn engine(&self) -> Arc<RwLock<MetabolismCycleEngine>> {
        Arc::clone(&self.engine)
    }

    /// Get a reference to the metrics for REST API integration.
    pub fn metrics(&self) -> Arc<AtomicMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Get the event broadcast sender for integration with other systems.
    /// This allows GraphQL subscriptions, SSE, and webhooks to receive events.
    pub fn event_sender(&self) -> broadcast::Sender<String> {
        self.event_tx.clone()
    }

    /// Start the WebSocket server.
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!(
            address = %self.config.bind_addr,
            trace_id = %trace_id_field(),
            "WebSocket server listening"
        );

        // Start health/metrics HTTP server if configured
        if let Some(health_addr) = self.config.health_addr {
            let metrics = Arc::clone(&self.metrics);
            let engine = Arc::clone(&self.engine);
            tokio::spawn(async move {
                if let Err(e) = Self::run_health_server(health_addr, metrics, engine).await {
                    error!(error = %e, "Health server error");
                }
            });
            info!(address = %health_addr, "Health/metrics server listening");
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
                            let ip = addr.ip();

                            // Check rate limits before accepting connection
                            let rate_limit_result = self.rate_limiter.register_connection(ip).await;
                            if !rate_limit_result.is_allowed() {
                                warn!(
                                    peer_addr = %addr,
                                    reason = ?rate_limit_result,
                                    "Connection rejected by rate limiter"
                                );
                                // Send rejection and close
                                Self::reject_connection(stream, rate_limit_result.error_message().unwrap_or("Rate limited")).await;
                                continue;
                            }

                            let connection_id = self.metrics.total_connections.fetch_add(1, Ordering::Relaxed) + 1;
                            self.metrics.active_connections.fetch_add(1, Ordering::Relaxed);

                            let connection_span = info_span!(
                                "ws_connection",
                                connection_id = connection_id,
                                peer_addr = %addr,
                                trace_id = %trace_id_field()
                            );

                            info!(
                                parent: &connection_span,
                                connection_id = connection_id,
                                peer_addr = %addr,
                                "New WebSocket connection"
                            );

                            let engine = Arc::clone(&self.engine);
                            let event_rx = self.event_tx.subscribe();
                            let metrics = Arc::clone(&self.metrics);
                            let rate_limiter = Arc::clone(&self.rate_limiter);
                            let authenticator = Arc::clone(&self.authenticator);

                            tokio::spawn(
                                async move {
                                    let result = Self::handle_connection_with_security(
                                        stream, addr, engine, event_rx, metrics, connection_id,
                                        rate_limiter, authenticator
                                    ).await;
                                    if let Err(e) = result {
                                        error!(
                                            connection_id = connection_id,
                                            error = %e,
                                            "Connection error"
                                        );
                                    }
                                }
                                .instrument(connection_span)
                            );
                        }
                        Err(e) => {
                            error!(error = %e, "Accept error");
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutdown signal received, gracefully shutting down...");
                    break;
                }
            }

            // Periodically clean up stale rate limit entries
            self.rate_limiter.cleanup_stale_entries().await;
        }

        // Graceful shutdown: wait for connections to drain
        info!("Waiting for active connections to close...");
        let mut wait_count = 0;
        while self.metrics.active_connections.load(Ordering::Relaxed) > 0 && wait_count < 30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            wait_count += 1;
            info!(
                active_connections = self.metrics.active_connections.load(Ordering::Relaxed),
                "Draining connections"
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
                let (status, content_type, body) = if request.starts_with("GET /health") {
                    ("200 OK", "application/json", r#"{"status":"healthy"}"#.to_string())
                } else if request.starts_with("GET /metrics/json") {
                    // JSON metrics endpoint for backwards compatibility
                    let m = ServerMetrics {
                        active_connections: metrics.active_connections.load(Ordering::Relaxed),
                        total_connections: metrics.total_connections.load(Ordering::Relaxed),
                        messages_received: metrics.messages_received.load(Ordering::Relaxed),
                        messages_sent: metrics.messages_sent.load(Ordering::Relaxed),
                        uptime_seconds: 0, // Can't easily get this here
                    };
                    ("200 OK", "application/json", serde_json::to_string(&m).unwrap_or_default())
                } else if request.starts_with("GET /metrics") {
                    // Prometheus/OpenMetrics format
                    let engine = engine.read().await;
                    let prometheus_metrics = Self::format_prometheus_metrics(&metrics, &engine);
                    ("200 OK", "text/plain; version=0.0.4; charset=utf-8", prometheus_metrics)
                } else if request.starts_with("GET /state") {
                    let engine = engine.read().await;
                    let state = CycleStateResponse {
                        cycle_number: engine.cycle_number(),
                        current_phase: engine.current_phase(),
                        phase_started: engine.phase_started().to_rfc3339(),
                        cycle_started: engine.cycle_started().to_rfc3339(),
                        phase_day: engine.phase_day(),
                    };
                    ("200 OK", "application/json", serde_json::to_string(&state).unwrap_or_default())
                } else {
                    ("404 Not Found", "application/json", r#"{"error":"not found"}"#.to_string())
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    content_type,
                    body.len(),
                    body
                );

                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    }

    /// Format metrics in Prometheus/OpenMetrics exposition format.
    fn format_prometheus_metrics(
        metrics: &Arc<AtomicMetrics>,
        engine: &MetabolismCycleEngine,
    ) -> String {
        let active_connections = metrics.active_connections.load(Ordering::Relaxed);
        let total_connections = metrics.total_connections.load(Ordering::Relaxed);
        let messages_received = metrics.messages_received.load(Ordering::Relaxed);
        let messages_sent = metrics.messages_sent.load(Ordering::Relaxed);
        let cycle_number = engine.cycle_number();
        let current_phase = engine.current_phase();

        // Map phase to numeric value for Prometheus gauge
        let phase_value = match current_phase {
            CyclePhase::Shadow => 0,
            CyclePhase::Composting => 1,
            CyclePhase::Liminal => 2,
            CyclePhase::NegativeCapability => 3,
            CyclePhase::Eros => 4,
            CyclePhase::CoCreation => 5,
            CyclePhase::Beauty => 6,
            CyclePhase::EmergentPersonhood => 7,
            CyclePhase::Kenosis => 8,
        };

        let phase_name = format!("{:?}", current_phase);
        let phase_day = engine.phase_day();

        format!(
            r#"# HELP mycelix_active_connections Current number of active WebSocket connections
# TYPE mycelix_active_connections gauge
mycelix_active_connections {active_connections}

# HELP mycelix_total_connections_total Total number of WebSocket connections since server start
# TYPE mycelix_total_connections_total counter
mycelix_total_connections_total {total_connections}

# HELP mycelix_messages_received_total Total messages received from clients
# TYPE mycelix_messages_received_total counter
mycelix_messages_received_total {messages_received}

# HELP mycelix_messages_sent_total Total messages sent to clients
# TYPE mycelix_messages_sent_total counter
mycelix_messages_sent_total {messages_sent}

# HELP mycelix_messages_total Total messages processed (received + sent)
# TYPE mycelix_messages_total counter
mycelix_messages_total {messages_total}

# HELP mycelix_cycle_number Current metabolism cycle number
# TYPE mycelix_cycle_number gauge
mycelix_cycle_number {cycle_number}

# HELP mycelix_cycle_phase Current phase in the metabolism cycle (0=Shadow, 1=Composting, 2=Liminal, 3=NegativeCapability, 4=Eros, 5=CoCreation, 6=Beauty, 7=EmergentPersonhood, 8=Kenosis)
# TYPE mycelix_cycle_phase gauge
mycelix_cycle_phase {phase_value}

# HELP mycelix_cycle_phase_info Current phase information with labels
# TYPE mycelix_cycle_phase_info gauge
mycelix_cycle_phase_info{{phase="{phase_name}",cycle="{cycle_number}"}} 1

# HELP mycelix_phase_day Current day within the current phase
# TYPE mycelix_phase_day gauge
mycelix_phase_day {phase_day}

# HELP mycelix_server_info Server information
# TYPE mycelix_server_info gauge
mycelix_server_info{{version="1.0.0"}} 1
"#,
            active_connections = active_connections,
            total_connections = total_connections,
            messages_received = messages_received,
            messages_sent = messages_sent,
            messages_total = messages_received + messages_sent,
            cycle_number = cycle_number,
            phase_value = phase_value,
            phase_name = phase_name,
            phase_day = phase_day,
        )
    }

    /// Reject a connection with an error message (before WebSocket upgrade).
    async fn reject_connection(mut stream: TcpStream, reason: &str) {
        let response = format!(
            "HTTP/1.1 429 Too Many Requests\r\n\
             Content-Type: text/plain\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            reason.len(),
            reason
        );
        let _ = stream.write_all(response.as_bytes()).await;
    }

    /// Reject a connection with authentication error.
    async fn reject_auth(mut stream: TcpStream, reason: &str) {
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\n\
             Content-Type: text/plain\r\n\
             WWW-Authenticate: ApiKey\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            reason.len(),
            reason
        );
        let _ = stream.write_all(response.as_bytes()).await;
    }

    /// Handle connection with security checks (rate limiting and authentication).
    async fn handle_connection_with_security(
        stream: TcpStream,
        addr: SocketAddr,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        event_rx: broadcast::Receiver<String>,
        metrics: Arc<AtomicMetrics>,
        connection_id: u64,
        rate_limiter: SharedRateLimiter,
        authenticator: SharedAuthenticator,
    ) -> anyhow::Result<()> {
        let ip = addr.ip();

        // We need to peek at the HTTP upgrade request to extract auth credentials
        // This is a simplified approach - we read the HTTP headers before the WS upgrade
        let mut buf_reader = BufReader::new(stream);
        let mut headers = String::new();
        let mut path = String::new();

        // Read HTTP request line and headers
        loop {
            let mut line = String::new();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if line == "\r\n" || line == "\n" {
                        break; // End of headers
                    }
                    if line.starts_with("GET ") || line.starts_with("get ") {
                        // Extract path from request line
                        if let Some(p) = line.split_whitespace().nth(1) {
                            path = p.to_string();
                        }
                    }
                    headers.push_str(&line);
                }
                Err(_) => break,
            }
        }

        // Extract credentials from headers and query string
        let query_string = path.split_once('?').map(|(_, qs)| qs);
        let credentials = AuthCredentials::from_request(&headers, query_string);

        // Authenticate
        let auth_result = authenticator.authenticate(&credentials);
        if !auth_result.is_allowed() {
            warn!(
                connection_id = connection_id,
                peer_addr = %addr,
                reason = ?auth_result.error_message(),
                "Authentication failed"
            );
            // Get the inner stream back and reject
            let stream = buf_reader.into_inner();
            Self::reject_auth(stream, auth_result.error_message().unwrap_or("Unauthorized")).await;
            rate_limiter.unregister_connection(ip).await;
            metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
            return Ok(());
        }

        if let Some(identity) = auth_result.identity() {
            debug!(
                connection_id = connection_id,
                identity = identity,
                "Authenticated connection"
            );
        }

        // Proceed with WebSocket connection
        let stream = buf_reader.into_inner();
        let result = Self::handle_connection_with_rate_limit(
            stream, addr, engine, event_rx, metrics.clone(), connection_id, rate_limiter.clone()
        ).await;

        // Cleanup
        rate_limiter.unregister_connection(ip).await;
        metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
        info!(
            connection_id = connection_id,
            peer_addr = %addr,
            "Connection closed"
        );

        result
    }

    /// Handle connection with rate limiting on requests.
    async fn handle_connection_with_rate_limit(
        stream: TcpStream,
        addr: SocketAddr,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        event_rx: broadcast::Receiver<String>,
        metrics: Arc<AtomicMetrics>,
        connection_id: u64,
        rate_limiter: SharedRateLimiter,
    ) -> anyhow::Result<()> {
        Self::handle_connection(stream, addr, engine, event_rx, metrics, connection_id, rate_limiter).await
    }

    /// Handle connection with metrics tracking.
    #[allow(dead_code)]
    async fn handle_connection_with_metrics(
        stream: TcpStream,
        addr: SocketAddr,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        event_rx: broadcast::Receiver<String>,
        metrics: Arc<AtomicMetrics>,
        connection_id: u64,
    ) -> anyhow::Result<()> {
        let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::default()));
        let result = Self::handle_connection(stream, addr, engine, event_rx, metrics.clone(), connection_id, rate_limiter).await;
        metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
        info!(
            connection_id = connection_id,
            peer_addr = %addr,
            "Connection closed"
        );
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
                        warn!(error = %e, "Tick error");
                        continue;
                    }
                }
            };

            // Broadcast events to all connected clients
            for event in events {
                // Create a span for phase transitions
                if let LivingProtocolEvent::PhaseTransitioned(ref transition_event) = event {
                    let transition = &transition_event.transition;
                    let _span = info_span!(
                        "phase_transition",
                        from_phase = ?transition.from,
                        to_phase = ?transition.to,
                        cycle_number = transition.cycle_number,
                        trace_id = %trace_id_field()
                    )
                    .entered();

                    info!(
                        from_phase = ?transition.from,
                        to_phase = ?transition.to,
                        cycle_number = transition.cycle_number,
                        "Phase transitioned"
                    );
                }

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
        metrics: Arc<AtomicMetrics>,
        connection_id: u64,
        rate_limiter: SharedRateLimiter,
    ) -> anyhow::Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();
        let ip = addr.ip();

        loop {
            tokio::select! {
                // Handle incoming messages from client
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Check rate limit for this request
                            let rate_check = rate_limiter.check_request(ip).await;
                            if !rate_check.is_allowed() {
                                warn!(
                                    connection_id = connection_id,
                                    peer_addr = %addr,
                                    "Request rate limited"
                                );
                                // Send rate limit error response
                                let error_response = serde_json::json!({
                                    "error": {
                                        "code": -32000,
                                        "message": rate_check.error_message().unwrap_or("Rate limited")
                                    }
                                });
                                if let Ok(json) = serde_json::to_string(&error_response) {
                                    let _ = write.send(Message::Text(json)).await;
                                }
                                continue;
                            }

                            metrics.messages_received.fetch_add(1, Ordering::Relaxed);
                            debug!(
                                connection_id = connection_id,
                                message = %text,
                                "Received message"
                            );

                            // Try to parse as RPC request
                            match serde_json::from_str::<RpcRequest>(&text) {
                                Ok(request) => {
                                    let request_span = info_span!(
                                        "rpc_request",
                                        connection_id = connection_id,
                                        request_id = %request.id,
                                        method = %request.method,
                                        trace_id = %trace_id_field()
                                    );

                                    let response = async {
                                        debug!(
                                            request_id = %request.id,
                                            method = %request.method,
                                            "Processing RPC request"
                                        );
                                        Self::handle_request(&request, &engine).await
                                    }
                                    .instrument(request_span)
                                    .await;

                                    if let Ok(json) = serde_json::to_string(&response) {
                                        write.send(Message::Text(json)).await?;
                                        metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                                Err(e) => {
                                    // Check if it's a ping
                                    if let Ok(obj) = serde_json::from_str::<Value>(&text) {
                                        if obj.get("type").and_then(|v| v.as_str()) == Some("ping") {
                                            let pong = serde_json::json!({"type": "pong"});
                                            write.send(Message::Text(pong.to_string())).await?;
                                            metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                                            continue;
                                        }
                                    }
                                    warn!(
                                        connection_id = connection_id,
                                        error = %e,
                                        "Failed to parse request"
                                    );
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!(
                                connection_id = connection_id,
                                peer_addr = %addr,
                                "Client disconnected"
                            );
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Err(e)) => {
                            error!(
                                connection_id = connection_id,
                                error = %e,
                                "WebSocket error"
                            );
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
                                error!(
                                    connection_id = connection_id,
                                    error = %e,
                                    "Failed to send event"
                                );
                                break;
                            }
                            metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(
                                connection_id = connection_id,
                                lagged_events = n,
                                "Client lagged behind"
                            );
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
