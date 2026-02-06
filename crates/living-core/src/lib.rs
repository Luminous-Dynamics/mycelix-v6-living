//! # Living Core
//!
//! Shared types, events, and traits for the Mycelix v6.0 Living Protocol Layer.
//! All 21 primitives and the Metabolism Cycle orchestrator depend on these types.

pub mod config;
pub mod error;
pub mod events;
pub mod k_vector;
pub mod traits;
pub mod types;

pub use config::*;
pub use error::*;
pub use events::*;
pub use k_vector::*;
pub use traits::*;
pub use types::*;
