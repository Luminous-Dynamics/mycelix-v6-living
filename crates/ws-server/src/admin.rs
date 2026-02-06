//! Admin panel server for the Living Protocol.
//!
//! Provides:
//! - Static file serving for the React admin panel
//! - Admin-specific REST API endpoints
//! - Basic authentication support

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use cycle_engine::MetabolismCycleEngine;
use living_core::CyclePhase;

use crate::server::AtomicMetrics;

/// Admin server configuration.
#[derive(Debug, Clone)]
pub struct AdminConfig {
    /// Address to bind the admin server to
    pub bind_addr: SocketAddr,
    /// Password for basic auth (username is always "admin")
    pub password: Option<String>,
    /// Whether test mode is enabled (allows cycle advancement)
    pub test_mode: bool,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8891".parse().unwrap(),
            password: None,
            test_mode: false,
        }
    }
}

/// Connection info for the admin API.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub id: u64,
    pub remote_addr: String,
    pub connected_at: String,
    pub authenticated: bool,
    pub messages_received: u64,
    pub messages_sent: u64,
}

/// Server configuration response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfigResponse {
    pub bind_addr: String,
    pub health_addr: Option<String>,
    pub max_connections: u32,
    pub max_connections_per_ip: u32,
    pub rate_limit: u32,
    pub rate_limit_burst: u32,
    pub auth_required: bool,
    pub test_mode: bool,
}

/// Admin API server.
pub struct AdminServer {
    config: AdminConfig,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    metrics: Arc<AtomicMetrics>,
    connections: Arc<RwLock<Vec<ConnectionInfo>>>,
    server_config: ServerConfigResponse,
}

impl AdminServer {
    /// Create a new admin server.
    pub fn new(
        config: AdminConfig,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        metrics: Arc<AtomicMetrics>,
        server_config: ServerConfigResponse,
    ) -> Self {
        Self {
            config,
            engine,
            metrics,
            connections: Arc::new(RwLock::new(Vec::new())),
            server_config,
        }
    }

    /// Run the admin server.
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!(
            address = %self.config.bind_addr,
            "Admin server listening"
        );

        loop {
            let (stream, addr) = listener.accept().await?;
            debug!(peer_addr = %addr, "Admin connection");

            let engine = Arc::clone(&self.engine);
            let metrics = Arc::clone(&self.metrics);
            let connections = Arc::clone(&self.connections);
            let config = self.config.clone();
            let server_config = self.server_config.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_request(
                    stream,
                    engine,
                    metrics,
                    connections,
                    config,
                    server_config,
                )
                .await
                {
                    error!(error = %e, "Admin request error");
                }
            });
        }
    }

    /// Handle an HTTP request.
    async fn handle_request(
        mut stream: TcpStream,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        metrics: Arc<AtomicMetrics>,
        connections: Arc<RwLock<Vec<ConnectionInfo>>>,
        config: AdminConfig,
        server_config: ServerConfigResponse,
    ) -> anyhow::Result<()> {
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }

        let request = String::from_utf8_lossy(&buf[..n]);
        let lines: Vec<&str> = request.lines().collect();

        if lines.is_empty() {
            return Ok(());
        }

        // Parse request line
        let parts: Vec<&str> = lines[0].split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let method = parts[0];
        let path = parts[1];

        // Parse headers
        let mut headers = HashMap::new();
        for line in &lines[1..] {
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim().to_lowercase(), value.trim().to_string());
            }
        }

        // Check authentication
        if let Some(ref password) = config.password {
            let authorized = if let Some(auth) = headers.get("authorization") {
                if let Some(encoded) = auth.strip_prefix("Basic ") {
                    if let Ok(decoded) = base64_decode(encoded) {
                        decoded == format!("admin:{}", password)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !authorized && !path.starts_with("/admin/api/") {
                // Serve index.html for the admin panel without auth
                // But protect API endpoints
            }

            if !authorized && path.starts_with("/admin/api/") {
                let response = "HTTP/1.1 401 Unauthorized\r\n\
                    WWW-Authenticate: Basic realm=\"Mycelix Admin\"\r\n\
                    Content-Type: text/plain\r\n\
                    Content-Length: 12\r\n\
                    Connection: close\r\n\r\nUnauthorized";
                stream.write_all(response.as_bytes()).await?;
                return Ok(());
            }
        }

        // Route the request
        let (status, content_type, body) = match (method, path) {
            // API endpoints
            ("GET", "/admin/api/state") => {
                let engine = engine.read().await;
                let state = CycleStateResponse {
                    cycle_number: engine.cycle_number(),
                    current_phase: engine.current_phase(),
                    phase_started: engine.phase_started().to_rfc3339(),
                    cycle_started: engine.cycle_started().to_rfc3339(),
                    phase_day: engine.phase_day(),
                };
                (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&state)?,
                )
            }

            ("GET", "/admin/api/connections") => {
                let conns = connections.read().await;
                (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&*conns)?,
                )
            }

            ("GET", "/admin/api/server/metrics") => {
                let m = ServerMetricsResponse {
                    active_connections: metrics.active_connections.load(Ordering::Relaxed),
                    total_connections: metrics.total_connections.load(Ordering::Relaxed),
                    messages_received: metrics.messages_received.load(Ordering::Relaxed),
                    messages_sent: metrics.messages_sent.load(Ordering::Relaxed),
                    uptime_seconds: 0, // Would need start time to calculate
                };
                (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&m)?,
                )
            }

            ("GET", "/admin/api/metrics") => {
                let engine = engine.read().await;
                let m = engine.phase_metrics(engine.current_phase());
                (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&m)?,
                )
            }

            ("GET", p) if p.starts_with("/admin/api/metrics/") => {
                let phase_str = &p[19..];
                if let Some(phase) = parse_phase(phase_str) {
                    let engine = engine.read().await;
                    let m = engine.phase_metrics(phase);
                    (
                        "200 OK",
                        "application/json",
                        serde_json::to_string(&m)?,
                    )
                } else {
                    (
                        "400 Bad Request",
                        "application/json",
                        r#"{"error":"Invalid phase"}"#.to_string(),
                    )
                }
            }

            ("GET", "/admin/api/history") => {
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
                (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&history)?,
                )
            }

            ("GET", "/admin/api/config") => (
                "200 OK",
                "application/json",
                serde_json::to_string(&server_config)?,
            ),

            ("POST", "/admin/api/cycle/advance") => {
                if !config.test_mode {
                    (
                        "403 Forbidden",
                        "application/json",
                        r#"{"error":"Cycle advancement only available in test mode"}"#.to_string(),
                    )
                } else {
                    let mut engine = engine.write().await;
                    match engine.force_transition() {
                        Ok(_) => {
                            let response = AdvanceResponse {
                                success: true,
                                new_phase: engine.current_phase(),
                            };
                            (
                                "200 OK",
                                "application/json",
                                serde_json::to_string(&response)?,
                            )
                        }
                        Err(e) => (
                            "500 Internal Server Error",
                            "application/json",
                            format!(r#"{{"error":"{}"}}"#, e),
                        ),
                    }
                }
            }

            // Static file serving for admin panel
            ("GET", "/") | ("GET", "/admin") | ("GET", "/admin/") => {
                serve_static_file("index.html")
            }

            ("GET", p) if p.starts_with("/assets/") || p.starts_with("/admin/assets/") => {
                let filename = if let Some(stripped) = p.strip_prefix("/admin/") {
                    stripped // Remove /admin prefix
                } else if let Some(stripped) = p.strip_prefix("/") {
                    stripped // Remove leading /
                } else {
                    p
                };
                serve_static_file(filename)
            }

            // SPA fallback - serve index.html for all other routes
            ("GET", p)
                if !p.starts_with("/admin/api/")
                    && (p.starts_with("/admin/") || !p.contains('.')) =>
            {
                serve_static_file("index.html")
            }

            _ => (
                "404 Not Found",
                "application/json",
                r#"{"error":"Not found"}"#.to_string(),
            ),
        };

        // Send response
        let response = format!(
            "HTTP/1.1 {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Connection: close\r\n\r\n{}",
            status,
            content_type,
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}

// Response types
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CycleStateResponse {
    cycle_number: u64,
    current_phase: CyclePhase,
    phase_started: String,
    cycle_started: String,
    phase_day: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PhaseTransitionResponse {
    from: CyclePhase,
    to: CyclePhase,
    cycle_number: u64,
    transitioned_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerMetricsResponse {
    active_connections: u64,
    total_connections: u64,
    messages_received: u64,
    messages_sent: u64,
    uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdvanceResponse {
    success: bool,
    new_phase: CyclePhase,
}

/// Parse a phase name string into a CyclePhase.
fn parse_phase(s: &str) -> Option<CyclePhase> {
    match s {
        "Shadow" => Some(CyclePhase::Shadow),
        "Composting" => Some(CyclePhase::Composting),
        "Liminal" => Some(CyclePhase::Liminal),
        "NegativeCapability" => Some(CyclePhase::NegativeCapability),
        "Eros" => Some(CyclePhase::Eros),
        "CoCreation" => Some(CyclePhase::CoCreation),
        "Beauty" => Some(CyclePhase::Beauty),
        "EmergentPersonhood" => Some(CyclePhase::EmergentPersonhood),
        "Kenosis" => Some(CyclePhase::Kenosis),
        _ => None,
    }
}

/// Simple base64 decode (for basic auth).
fn base64_decode(input: &str) -> Result<String, ()> {
    const DECODE_TABLE: [i8; 256] = {
        let mut table = [-1i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as i8;
            i += 1;
        }
        table
    };

    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    let bytes = input.as_bytes();
    let chunks = bytes.chunks(4);

    for chunk in chunks {
        let mut val = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            let decoded = DECODE_TABLE[b as usize];
            if decoded < 0 {
                return Err(());
            }
            val |= (decoded as u32) << (18 - i * 6);
        }

        output.push((val >> 16) as u8);
        if chunk.len() > 2 {
            output.push((val >> 8) as u8);
        }
        if chunk.len() > 3 {
            output.push(val as u8);
        }
    }

    String::from_utf8(output).map_err(|_| ())
}

/// Serve a static file from embedded assets.
///
/// In development, files are served from the filesystem.
/// In production, files are embedded in the binary.
fn serve_static_file(filename: &str) -> (&'static str, &'static str, String) {
    // For now, return a placeholder HTML that loads from the dev server
    // In production, this would use rust-embed to serve the built files
    if filename == "index.html" {
        let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mycelix Admin Panel</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            background: #1a1a2e;
            color: #e0e0e0;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
        }
        .container {
            text-align: center;
            padding: 2rem;
        }
        h1 { color: #0ea5e9; }
        p { color: #9ca3af; }
        code {
            background: #374151;
            padding: 0.25rem 0.5rem;
            border-radius: 0.25rem;
            font-size: 0.875rem;
        }
        .status {
            margin-top: 2rem;
            padding: 1rem;
            background: #374151;
            border-radius: 0.5rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Mycelix Admin Panel</h1>
        <p>The admin panel is not built yet.</p>
        <p>To start the development server:</p>
        <p><code>cd admin && npm install && npm run dev</code></p>
        <div class="status">
            <p>Admin API is running at this address.</p>
            <p>API endpoints available under <code>/admin/api/</code></p>
        </div>
    </div>
</body>
</html>"#;
        ("200 OK", "text/html", html.to_string())
    } else {
        (
            "404 Not Found",
            "text/plain",
            "File not found".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase() {
        assert_eq!(parse_phase("Shadow"), Some(CyclePhase::Shadow));
        assert_eq!(parse_phase("CoCreation"), Some(CyclePhase::CoCreation));
        assert_eq!(parse_phase("Invalid"), None);
    }

    #[test]
    fn test_base64_decode() {
        assert_eq!(base64_decode("YWRtaW46cGFzcw=="), Ok("admin:pass".to_string()));
        assert_eq!(base64_decode("dGVzdA=="), Ok("test".to_string()));
    }
}
