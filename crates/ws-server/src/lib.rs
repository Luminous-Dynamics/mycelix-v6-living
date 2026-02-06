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
//! # Admin Panel
//!
//! When enabled with `--enable-admin`, provides a web-based admin panel:
//!
//! - Dashboard with key metrics
//! - Cycle control (in test mode)
//! - Connection monitoring
//! - Phase transition history
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
//!
//! # With admin panel
//! cargo run -p ws-server -- --enable-admin --admin-port 8891
//! ```

pub mod admin;
pub mod auth;
pub mod rate_limit;
pub mod rest;
mod rpc;
mod server;
pub mod telemetry;

// GraphQL API (requires graphql feature)
#[cfg(feature = "graphql")]
pub mod graphql;

// Server-Sent Events (requires sse feature)
#[cfg(feature = "sse")]
pub mod sse;

// Webhooks (requires webhooks feature)
#[cfg(feature = "webhooks")]
pub mod webhooks;

// Persistence layer (requires sqlite or postgres feature)
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod persistence;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub mod repository;

pub use admin::{AdminConfig, AdminServer, ServerConfigResponse};
pub use auth::{AuthConfig, AuthCredentials, AuthResult, Authenticator};
pub use rate_limit::{RateLimitConfig, RateLimitResult, RateLimiter};
pub use rest::{RestConfig, RestServer};
pub use rpc::{
    RpcError, RpcRequest, RpcResponse, RPC_ERROR_INTERNAL, RPC_ERROR_INVALID_PARAMS,
    RPC_ERROR_METHOD_NOT_FOUND,
};
pub use server::{AtomicMetrics, ServerConfig, ServerMetrics, WebSocketServer};

// GraphQL exports
#[cfg(feature = "graphql")]
pub use graphql::{
    create_schema, run_graphql_server, GraphQLConfig, GraphQLSchema,
    Phase, CycleState as GraphQLCycleState, PhaseMetrics as GraphQLPhaseMetrics,
    PhaseTransition as GraphQLPhaseTransition,
};

// SSE exports
#[cfg(feature = "sse")]
pub use sse::{create_sse_router, run_sse_server, SseConfig};

// Webhook exports
#[cfg(feature = "webhooks")]
pub use webhooks::{
    parse_webhook_events, DeliveryResult, RetryConfig, WebhookConfig, WebhookManager,
    WebhookPayload,
};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use persistence::{DatabasePool, PersistenceConfig, PersistenceError, PersistenceResult};
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub use repository::{CycleRepository, create_repository};
#[cfg(feature = "sqlite")]
pub use repository::SqliteRepository;
#[cfg(feature = "postgres")]
pub use repository::PostgresRepository;
