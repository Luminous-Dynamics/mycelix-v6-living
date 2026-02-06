//! WebSocket RPC Server for Mycelix Living Protocol
//!
//! Provides a WebSocket server that handles JSON-RPC style requests from
//! TypeScript clients and broadcasts protocol events.
//!
//! # Protocol
//!
//! ## Requests (client -> server)
//! ```json
//! { "id": "1", "method": "getCycleState", "params": {} }
//! ```
//!
//! ## Responses (server -> client)
//! ```json
//! { "id": "1", "result": { "cycleNumber": 1, "currentPhase": "Shadow", ... } }
//! { "id": "1", "error": { "code": -32601, "message": "Method not found" } }
//! ```
//!
//! ## Events (server -> client, no id)
//! ```json
//! { "type": "PhaseTransitioned", "data": { "from": "Shadow", "to": "Composting" } }
//! ```
//!
//! # REST API
//!
//! When enabled with `--enable-rest`, provides HTTP REST endpoints:
//!
//! - `GET /api/v1/state` - Current cycle state
//! - `GET /api/v1/phase` - Current phase
//! - `GET /api/v1/history` - Transition history
//! - `GET /api/v1/metrics/:phase` - Phase-specific metrics
//!
//! # Telemetry
//!
//! This crate supports OpenTelemetry tracing and structured JSON logging.
//! Enable the `otlp` feature to export traces to an OTLP collector.
//!
//! ```bash
//! # With OTLP export
//! cargo run -p ws-server --features otlp -- --otlp-endpoint http://localhost:4317
//!
//! # With JSON logs
//! cargo run -p ws-server -- --json-logs
//! ```

pub mod auth;
pub mod rate_limit;
pub mod rest;
mod rpc;
mod server;
pub mod telemetry;

pub use auth::{AuthConfig, AuthCredentials, AuthResult, Authenticator};
pub use rate_limit::{RateLimitConfig, RateLimitResult, RateLimiter};
pub use rest::{RestConfig, RestServer};
pub use rpc::{
    RpcError, RpcRequest, RpcResponse, RPC_ERROR_INTERNAL, RPC_ERROR_INVALID_PARAMS,
    RPC_ERROR_METHOD_NOT_FOUND,
};
pub use server::{ServerConfig, WebSocketServer};
