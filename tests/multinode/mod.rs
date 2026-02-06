//! Multi-node Holochain testing framework for Mycelix v6.0.
//!
//! This module provides infrastructure for testing across multiple Holochain
//! conductors to verify:
//! - Link propagation and consistency
//! - Concurrent operation handling
//! - Network partition resilience
//!
//! # Running Multi-node Tests
//!
//! ```bash
//! cargo test --test multinode -- --test-threads=1
//! ```
//!
//! Note: Multi-node tests require `--test-threads=1` to avoid conductor conflicts.

pub mod consistency;
pub mod concurrency;
pub mod partition;

use std::sync::Arc;
use std::time::Duration;

use holochain::sweettest::*;
use hdk::prelude::*;

// =============================================================================
// Test Configuration
// =============================================================================

/// Default gossip wait time for link propagation (5 seconds).
pub const DEFAULT_GOSSIP_WAIT: Duration = Duration::from_secs(5);

/// Extended gossip wait time for slower networks (10 seconds).
pub const EXTENDED_GOSSIP_WAIT: Duration = Duration::from_secs(10);

/// Test network configuration.
#[derive(Clone, Debug)]
pub struct TestNetworkConfig {
    /// Number of conductors in the network.
    pub conductor_count: usize,

    /// Time to wait for gossip propagation.
    pub gossip_wait: Duration,

    /// Whether to enable network simulation features.
    pub enable_network_sim: bool,
}

impl Default for TestNetworkConfig {
    fn default() -> Self {
        Self {
            conductor_count: 2,
            gossip_wait: DEFAULT_GOSSIP_WAIT,
            enable_network_sim: false,
        }
    }
}

impl TestNetworkConfig {
    /// Create a config for 2-node tests.
    pub fn two_node() -> Self {
        Self::default()
    }

    /// Create a config for 3-node tests.
    pub fn three_node() -> Self {
        Self {
            conductor_count: 3,
            ..Self::default()
        }
    }

    /// Create a config with extended gossip wait.
    pub fn with_extended_gossip(mut self) -> Self {
        self.gossip_wait = EXTENDED_GOSSIP_WAIT;
        self
    }
}

// =============================================================================
// Test Fixtures
// =============================================================================

/// A configured multi-node test network.
pub struct TestNetwork {
    /// The SweetConductorBatch containing all conductors.
    pub conductors: SweetConductorBatch,

    /// Configuration used to create this network.
    pub config: TestNetworkConfig,
}

impl TestNetwork {
    /// Set up a new test network with the given configuration.
    pub async fn setup(config: TestNetworkConfig) -> Self {
        let conductors = SweetConductorBatch::from_config(
            config.conductor_count,
            SweetConductorConfig::standard(),
        )
        .await;

        Self { conductors, config }
    }

    /// Set up a 2-node test network.
    pub async fn setup_2_node() -> Self {
        Self::setup(TestNetworkConfig::two_node()).await
    }

    /// Set up a 3-node test network.
    pub async fn setup_3_node() -> Self {
        Self::setup(TestNetworkConfig::three_node()).await
    }

    /// Get conductor by index.
    pub fn conductor(&self, index: usize) -> &SweetConductor {
        &self.conductors[index]
    }

    /// Wait for gossip to propagate.
    pub async fn wait_for_gossip(&self) {
        tokio::time::sleep(self.config.gossip_wait).await;
    }

    /// Wait for a custom duration.
    pub async fn wait(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

// =============================================================================
// Test Utilities
// =============================================================================

/// Wrapper for calling zome functions with better error messages.
pub async fn call_zome<I, O>(
    conductor: &SweetConductor,
    cell: &SweetCell,
    zome: &str,
    fn_name: &str,
    input: I,
) -> O
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    conductor
        .call(cell.zome(zome), fn_name, input)
        .await
}

/// Assert that a record exists on a conductor.
pub async fn assert_record_exists(
    conductor: &SweetConductor,
    cell: &SweetCell,
    action_hash: &ActionHash,
) {
    let result: Option<Record> = conductor
        .call(cell.zome("living_metabolism"), "get", action_hash.clone())
        .await;
    assert!(result.is_some(), "Record should exist: {:?}", action_hash);
}

/// Assert that a specific number of links exist from a base.
pub async fn assert_link_count(
    conductor: &SweetConductor,
    cell: &SweetCell,
    zome: &str,
    fn_name: &str,
    base: impl serde::Serialize + std::fmt::Debug,
    expected_count: usize,
) {
    let records: Vec<Record> = conductor
        .call(cell.zome(zome), fn_name, base)
        .await;
    assert_eq!(
        records.len(),
        expected_count,
        "Expected {} links, found {}",
        expected_count,
        records.len()
    );
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur in multi-node tests.
#[derive(Debug, Clone)]
pub enum MultiNodeTestError {
    /// Link not visible on expected conductor.
    LinkNotPropagated {
        base: String,
        expected_on: String,
    },

    /// Concurrent operation produced unexpected result.
    ConcurrencyViolation {
        operation: String,
        detail: String,
    },

    /// Network partition simulation failed.
    PartitionError {
        detail: String,
    },

    /// Timeout waiting for condition.
    Timeout {
        operation: String,
        waited: Duration,
    },
}

impl std::fmt::Display for MultiNodeTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LinkNotPropagated { base, expected_on } => {
                write!(f, "Link from {} not propagated to {}", base, expected_on)
            }
            Self::ConcurrencyViolation { operation, detail } => {
                write!(f, "Concurrency violation in {}: {}", operation, detail)
            }
            Self::PartitionError { detail } => {
                write!(f, "Partition error: {}", detail)
            }
            Self::Timeout { operation, waited } => {
                write!(f, "Timeout in {} after {:?}", operation, waited)
            }
        }
    }
}

impl std::error::Error for MultiNodeTestError {}
