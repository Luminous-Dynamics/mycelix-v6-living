//! # Metabolism Cycle Engine
//!
//! The central orchestrator for the Mycelix v6.0 Living Protocol Layer.
//! Manages the 28-day (lunar) metabolism cycle that connects all 21 primitives
//! into a coherent lifecycle.
//!
//! ## Cycle Phases (28 days total)
//!
//! ```text
//! Shadow (2d) → Composting (5d) → Liminal (3d) → Negative Capability (3d) →
//! Eros (4d) → Co-Creation (7d) → Beauty (2d) → Emergent Personhood (1d) →
//! Kenosis (1d) → [back to Shadow]
//! ```
//!
//! ## Plugin Architecture
//!
//! The engine supports plugins that can hook into the lifecycle:
//!
//! ```rust,ignore
//! use cycle_engine::{CycleEngineBuilder, plugin::{Plugin, PluginConfig}};
//!
//! let engine = CycleEngineBuilder::new()
//!     .with_plugin(Box::new(MyPlugin::new()), PluginConfig::enabled())
//!     .build();
//! ```
//!
//! ## Middleware Architecture
//!
//! RPC request/response interception is supported via middleware:
//!
//! ```rust,ignore
//! use cycle_engine::middleware::{MiddlewareChain, LoggingMiddleware, MetricsMiddleware};
//!
//! let mut chain = MiddlewareChain::new();
//! chain.add(LoggingMiddleware::new());
//! chain.add(MetricsMiddleware::new());
//! ```
//!
//! ## Telemetry (Optional)
//!
//! Enable the `telemetry` feature to add OpenTelemetry observability:
//!
//! ```toml
//! [dependencies]
//! cycle-engine = { path = "crates/cycle-engine", features = ["telemetry"] }
//! ```
//!
//! This provides:
//! - Distributed tracing via OTLP export
//! - Metrics: phase_transition_count, tick_duration_ms, cycle_completion_time
//! - Automatic span instrumentation on key scheduler methods
//!
//! ## Dynamic Plugin Loading (Optional)
//!
//! Enable the `dynamic-plugins` feature to load plugins from shared libraries:
//!
//! ```toml
//! [dependencies]
//! cycle-engine = { path = "crates/cycle-engine", features = ["dynamic-plugins"] }
//! ```

pub mod chaos;
pub mod engine;
pub mod middleware;
pub mod phase_handlers;
pub mod plugin;
pub mod scheduler;

/// OpenTelemetry observability integration.
///
/// Only available when the `telemetry` feature is enabled.
#[cfg(feature = "telemetry")]
pub mod telemetry;

#[cfg(test)]
mod fuzz;

pub use engine::*;
pub use middleware::*;
pub use phase_handlers::*;
pub use plugin::*;
pub use scheduler::*;
