//! REST API endpoints for the Living Protocol.
//!
//! Provides HTTP REST access alongside the WebSocket RPC interface.
//! Enable with the `--enable-rest` flag on the server.
//!
//! ## Endpoints
//!
//! - `GET /api/v1/state` - Current cycle state
//! - `GET /api/v1/phase` - Current phase name
//! - `GET /api/v1/history` - Transition history
//! - `GET /api/v1/metrics/:phase` - Phase-specific metrics
//! - `GET /health` - Health check
//! - `GET /metrics` - Server metrics

use std::net::SocketAddr;
use std::sync::Arc;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{error, info};

use cycle_engine::MetabolismCycleEngine;
use living_core::CyclePhase;

use crate::server::{AtomicMetrics, CycleStateResponse, PhaseTransitionResponse};

/// REST API server configuration.
#[derive(Debug, Clone)]
pub struct RestConfig {
    /// Address to bind to
    pub bind_addr: SocketAddr,
    /// CORS allowed origins (None = allow all)
    pub cors_origins: Option<Vec<String>>,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8889".parse().unwrap(),
            cors_origins: None,
        }
    }
}

/// REST API response wrapper.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Phase response for the `/api/v1/phase` endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseResponse {
    pub phase: CyclePhase,
    pub duration_days: u32,
    pub phase_day: u32,
}

/// History response for the `/api/v1/history` endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    pub transitions: Vec<PhaseTransitionResponse>,
    pub total_count: usize,
}

/// Metrics response for the `/api/v1/metrics/:phase` endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseMetricsResponse {
    pub phase: CyclePhase,
    pub active_agents: u64,
    pub spectral_k: f64,
    pub mean_metabolic_trust: f64,
    pub active_wounds: u64,
    pub composting_entities: u64,
    pub liminal_entities: u64,
    pub entangled_pairs: u64,
    pub held_uncertainties: u64,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// REST API server.
pub struct RestServer {
    config: RestConfig,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    metrics: Arc<AtomicMetrics>,
}

impl RestServer {
    /// Create a new REST server.
    pub fn new(
        config: RestConfig,
        engine: Arc<RwLock<MetabolismCycleEngine>>,
        metrics: Arc<AtomicMetrics>,
    ) -> Self {
        Self {
            config,
            engine,
            metrics,
        }
    }

    /// Run the REST API server.
    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;
        info!("REST API server listening on {}", self.config.bind_addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            let engine = Arc::clone(&self.engine);
            let metrics = Arc::clone(&self.metrics);
            let cors_origins = self.config.cors_origins.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_request(stream, addr, engine, metrics, cors_origins).await {
                    error!("REST request error from {}: {}", addr, e);
                }
            });
        }
    }
}

/// Handle an incoming HTTP request.
async fn handle_request(
    mut stream: TcpStream,
    _addr: SocketAddr,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    metrics: Arc<AtomicMetrics>,
    cors_origins: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the request line
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    let (method, path) = if parts.len() >= 2 {
        (parts[0], parts[1])
    } else {
        ("", "")
    };

    // Route the request
    let (status, body) = match (method, path) {
        ("GET", "/health") => {
            let resp = HealthResponse {
                status: "healthy",
                version: env!("CARGO_PKG_VERSION"),
            };
            ("200 OK", serde_json::to_string(&resp).unwrap())
        }

        ("GET", "/api/v1/state") => {
            let engine = engine.read().await;
            let state = CycleStateResponse {
                cycle_number: engine.cycle_number(),
                current_phase: engine.current_phase(),
                phase_started: engine.phase_started().to_rfc3339(),
                cycle_started: engine.cycle_started().to_rfc3339(),
                phase_day: engine.phase_day(),
            };
            ("200 OK", serde_json::to_string(&ApiResponse::ok(state)).unwrap())
        }

        ("GET", "/api/v1/phase") => {
            let engine = engine.read().await;
            let resp = PhaseResponse {
                phase: engine.current_phase(),
                duration_days: engine.current_phase().duration_days(),
                phase_day: engine.phase_day(),
            };
            ("200 OK", serde_json::to_string(&ApiResponse::ok(resp)).unwrap())
        }

        ("GET", "/api/v1/history") => {
            let engine = engine.read().await;
            let transitions: Vec<PhaseTransitionResponse> = engine
                .transition_history()
                .iter()
                .map(|t| PhaseTransitionResponse {
                    from: t.from,
                    to: t.to,
                    cycle_number: t.cycle_number,
                    transitioned_at: t.transitioned_at.to_rfc3339(),
                })
                .collect();
            let resp = HistoryResponse {
                total_count: transitions.len(),
                transitions,
            };
            ("200 OK", serde_json::to_string(&ApiResponse::ok(resp)).unwrap())
        }

        ("GET", path) if path.starts_with("/api/v1/metrics/") => {
            let phase_str = path.strip_prefix("/api/v1/metrics/").unwrap_or("");
            match parse_phase(phase_str) {
                Ok(phase) => {
                    let engine = engine.read().await;
                    let m = engine.phase_metrics(phase);
                    let resp = PhaseMetricsResponse {
                        phase,
                        active_agents: m.active_agents,
                        spectral_k: m.spectral_k,
                        mean_metabolic_trust: m.mean_metabolic_trust,
                        active_wounds: m.active_wounds,
                        composting_entities: m.composting_entities,
                        liminal_entities: m.liminal_entities,
                        entangled_pairs: m.entangled_pairs,
                        held_uncertainties: m.held_uncertainties,
                    };
                    ("200 OK", serde_json::to_string(&ApiResponse::ok(resp)).unwrap())
                }
                Err(e) => {
                    let resp = ApiResponse::<()>::error(e);
                    ("400 Bad Request", serde_json::to_string(&resp).unwrap())
                }
            }
        }

        ("GET", "/metrics") => {
            use std::sync::atomic::Ordering;
            let m = crate::server::ServerMetrics {
                active_connections: metrics.active_connections.load(Ordering::Relaxed),
                total_connections: metrics.total_connections.load(Ordering::Relaxed),
                messages_received: metrics.messages_received.load(Ordering::Relaxed),
                messages_sent: metrics.messages_sent.load(Ordering::Relaxed),
                uptime_seconds: 0, // Would need to pass start time
            };
            ("200 OK", serde_json::to_string(&m).unwrap())
        }

        ("OPTIONS", _) => {
            // Handle CORS preflight
            ("204 No Content", String::new())
        }

        _ => {
            let resp = ApiResponse::<()>::error("Not found");
            ("404 Not Found", serde_json::to_string(&resp).unwrap())
        }
    };

    // Build CORS headers
    let cors_header = match cors_origins {
        Some(ref origins) if !origins.is_empty() => {
            format!("Access-Control-Allow-Origin: {}", origins.join(", "))
        }
        _ => "Access-Control-Allow-Origin: *".to_string(),
    };

    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         {}\r\n\
         Access-Control-Allow-Methods: GET, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Connection: close\r\n\r\n\
         {}",
        status,
        body.len(),
        cors_header,
        body
    );

    stream.write_all(response.as_bytes()).await?;

    Ok(())
}

/// Parse a phase string into a CyclePhase.
fn parse_phase(s: &str) -> Result<CyclePhase, String> {
    match s {
        "Shadow" => Ok(CyclePhase::Shadow),
        "Composting" => Ok(CyclePhase::Composting),
        "Liminal" => Ok(CyclePhase::Liminal),
        "NegativeCapability" => Ok(CyclePhase::NegativeCapability),
        "Eros" => Ok(CyclePhase::Eros),
        "CoCreation" => Ok(CyclePhase::CoCreation),
        "Beauty" => Ok(CyclePhase::Beauty),
        "EmergentPersonhood" => Ok(CyclePhase::EmergentPersonhood),
        "Kenosis" => Ok(CyclePhase::Kenosis),
        _ => Err(format!("Unknown phase: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phase() {
        assert_eq!(parse_phase("Shadow").unwrap(), CyclePhase::Shadow);
        assert_eq!(parse_phase("CoCreation").unwrap(), CyclePhase::CoCreation);
        assert!(parse_phase("Invalid").is_err());
    }

    #[test]
    fn test_api_response_ok() {
        let resp = ApiResponse::ok("test");
        assert!(resp.success);
        assert_eq!(resp.data, Some("test"));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let resp = ApiResponse::<()>::error("Something went wrong");
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert_eq!(resp.error, Some("Something went wrong".to_string()));
    }
}
