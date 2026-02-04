//! # Living Core
//!
//! Shared types, events, and traits for the Mycelix v6.0 Living Protocol Layer.
//! All 21 primitives and the Metabolism Cycle orchestrator depend on these types.

pub mod types;
pub mod events;
pub mod k_vector;
pub mod config;
pub mod traits;
pub mod error;

pub use types::*;
pub use events::*;
pub use k_vector::*;
pub use config::*;
pub use traits::*;
pub use error::*;
