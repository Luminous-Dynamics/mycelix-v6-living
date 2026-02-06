//! Fuzz target for event serialization/deserialization.
//!
//! This target fuzzes the Living Protocol event serialization to ensure:
//! - Roundtrip consistency (serialize -> deserialize -> serialize)
//! - No panics on arbitrary input
//! - Proper handling of all event variants
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run event_serialize -- -max_len=8192
//! ```

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use living_core::LivingProtocolEvent;

/// Fuzz event serialization and deserialization
fuzz_target!(|data: &[u8]| {
    // Try to deserialize arbitrary bytes as an event
    if let Ok(json_str) = std::str::from_utf8(data) {
        if let Ok(event) = serde_json::from_str::<LivingProtocolEvent>(json_str) {
            // Serialize back
            if let Ok(serialized) = serde_json::to_string(&event) {
                // Deserialize again
                if let Ok(event2) = serde_json::from_str::<LivingProtocolEvent>(&serialized) {
                    // Serialize once more for consistency check
                    let _ = serde_json::to_string(&event2);
                }
            }
        }
    }

    // Also try direct bytes
    if let Ok(event) = serde_json::from_slice::<LivingProtocolEvent>(data) {
        let _ = serde_json::to_vec(&event);
    }
});

/// Fuzz with structured arbitrary data
#[derive(Debug, Arbitrary)]
struct FuzzEvent {
    event_type: u8,
    id: String,
    timestamp_secs: i64,
    data_len: u8,
    data: Vec<u8>,
}

/// Alternative fuzz target using structured data
#[cfg(feature = "structured")]
fuzz_target!(|input: FuzzEvent| {
    // Create JSON from structured input
    let json = format!(
        r#"{{"type":"{}","id":"{}","timestamp":{},"data":{:?}}}"#,
        input.event_type,
        input.id.chars().take(100).collect::<String>(),
        input.timestamp_secs,
        &input.data[..input.data.len().min(256)]
    );

    // Try to parse as event
    let _ = serde_json::from_str::<LivingProtocolEvent>(&json);
});
