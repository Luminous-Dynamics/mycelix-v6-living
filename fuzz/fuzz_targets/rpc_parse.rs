//! Fuzz target for RPC request parsing.
//!
//! This target fuzzes the JSON-RPC request parsing logic to ensure:
//! - No panics on malformed input
//! - Memory safety with arbitrary byte sequences
//! - Proper error handling for invalid JSON
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run rpc_parse -- -max_len=4096
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use serde_json::Value;

/// Simplified RPC request structure matching ws-server/src/rpc.rs
#[derive(Debug, Clone, Deserialize)]
struct RpcRequest {
    /// Request ID for correlation
    pub id: String,
    /// Method name to invoke
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: Value,
}

/// Fuzz the RPC request parsing
fuzz_target!(|data: &[u8]| {
    // Try to parse as UTF-8 string first
    if let Ok(json_str) = std::str::from_utf8(data) {
        // Try to parse as RPC request
        let _ = serde_json::from_str::<RpcRequest>(json_str);

        // Also try partial parsing for error recovery
        let _ = serde_json::from_str::<Value>(json_str);

        // Try to parse as an array of requests (batch RPC)
        let _ = serde_json::from_str::<Vec<RpcRequest>>(json_str);
    }

    // Also try direct bytes parsing
    let _ = serde_json::from_slice::<RpcRequest>(data);
    let _ = serde_json::from_slice::<Value>(data);
});
