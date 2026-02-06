//! Fuzz target for K-Vector operations.
//!
//! This target fuzzes K-Vector operations to ensure:
//! - Mathematical invariants are maintained
//! - No panics or overflows
//! - Values remain bounded
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run kvector_operations -- -max_len=1024
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use chrono::{Duration, Utc};
use libfuzzer_sys::fuzz_target;
use living_core::{KVectorSignature, TemporalKVector};

/// K-Vector operations to fuzz
#[derive(Debug, Arbitrary)]
enum KVectorOp {
    /// Create new K-Vector with given values
    Create { values: [u16; 8] },
    /// Update temporal K-Vector
    Update { values: [u16; 8], days_forward: u8 },
    /// Compute distance
    Distance { other_values: [u16; 8] },
    /// Compute cosine similarity
    CosineSimilarity { other_values: [u16; 8] },
    /// Predict future values
    Predict { days: u8 },
    /// Check alignment
    CheckAlignment { other_values: [u16; 8] },
    /// Get magnitude
    Magnitude,
    /// Check sanity
    CheckSanity,
}

fn values_to_kvec(values: [u16; 8], days_ago: i64) -> KVectorSignature {
    // Map u16 to [0.0, 1.0]
    let arr: [f64; 8] = [
        (values[0] as f64) / 65535.0,
        (values[1] as f64) / 65535.0,
        (values[2] as f64) / 65535.0,
        (values[3] as f64) / 65535.0,
        (values[4] as f64) / 65535.0,
        (values[5] as f64) / 65535.0,
        (values[6] as f64) / 65535.0,
        (values[7] as f64) / 65535.0,
    ];
    KVectorSignature::from_array(arr, Utc::now() - Duration::days(days_ago))
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    // Get initial values
    let initial_values: [u16; 8] = match u.arbitrary() {
        Ok(v) => v,
        Err(_) => return,
    };

    let initial = values_to_kvec(initial_values, 10);
    let mut temporal = TemporalKVector::new(initial.clone(), 100);

    // Get number of operations
    let op_count: usize = match u.int_in_range(1..=20) {
        Ok(n) => n,
        Err(_) => return,
    };

    let mut current_time_offset: i64 = 10;

    for _ in 0..op_count {
        let op: KVectorOp = match u.arbitrary() {
            Ok(op) => op,
            Err(_) => break,
        };

        match op {
            KVectorOp::Create { values } => {
                let _kvec = values_to_kvec(values, current_time_offset);
            }
            KVectorOp::Update { values, days_forward } => {
                // Move time forward
                current_time_offset = current_time_offset.saturating_sub(days_forward as i64);
                if current_time_offset < 0 {
                    current_time_offset = 0;
                }
                let kvec = values_to_kvec(values, current_time_offset);
                temporal.update(kvec);

                // Check invariants
                for vel in temporal.velocity.iter() {
                    assert!(vel.is_finite(), "Velocity should be finite");
                }
                for acc in temporal.acceleration.iter() {
                    assert!(acc.is_finite(), "Acceleration should be finite");
                }
            }
            KVectorOp::Distance { other_values } => {
                let other = values_to_kvec(other_values, current_time_offset);
                let dist = initial.distance(&other);
                assert!(dist >= 0.0, "Distance should be non-negative");
                assert!(dist.is_finite(), "Distance should be finite");
            }
            KVectorOp::CosineSimilarity { other_values } => {
                let other = values_to_kvec(other_values, current_time_offset);
                let sim = initial.cosine_similarity(&other);
                assert!(sim.is_finite(), "Similarity should be finite");
                assert!(
                    sim >= -1.0 - 0.001 && sim <= 1.0 + 0.001,
                    "Similarity out of bounds: {}",
                    sim
                );
            }
            KVectorOp::Predict { days } => {
                let predicted = temporal.predict(days as f64);
                for (i, val) in predicted.iter().enumerate() {
                    assert!(
                        *val >= 0.0 && *val <= 1.0,
                        "Predicted value[{}] = {} out of bounds",
                        i,
                        val
                    );
                }
            }
            KVectorOp::CheckAlignment { other_values } => {
                let other = values_to_kvec(other_values, current_time_offset);
                let _aligned = initial.is_aligned_with(&other);
            }
            KVectorOp::Magnitude => {
                let mag = initial.magnitude();
                assert!(mag >= 0.0, "Magnitude should be non-negative");
                // Max magnitude = sqrt(8) when all dimensions are 1.0
                assert!(
                    mag <= (8.0_f64).sqrt() + 0.001,
                    "Magnitude exceeds maximum: {}",
                    mag
                );
            }
            KVectorOp::CheckSanity => {
                let _sane = initial.is_sane();
            }
        }
    }

    // Final invariant check on rate of change
    let roc = temporal.rate_of_change();
    assert!(roc >= 0.0, "Rate of change should be non-negative");
    assert!(roc.is_finite(), "Rate of change should be finite");
});
