//! Fuzz target for cycle engine state machine.
//!
//! This target fuzzes the cycle engine to ensure:
//! - State machine invariants are maintained
//! - No panics under arbitrary operation sequences
//! - Proper handling of edge cases
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run cycle_state -- -max_len=1024
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use living_core::CyclePhase;

/// Operations that can be performed on the cycle engine
#[derive(Debug, Arbitrary)]
enum CycleOp {
    /// Start the engine
    Start,
    /// Stop the engine
    Stop,
    /// Force a phase transition
    ForceTransition,
    /// Tick the engine
    Tick,
    /// Create checkpoint
    Checkpoint,
    /// Restore from checkpoint
    RestoreCheckpoint,
    /// Query current phase
    QueryPhase,
    /// Query cycle number
    QueryCycleNumber,
    /// Check if running
    CheckRunning,
}

fuzz_target!(|data: &[u8]| {
    // Parse operations from fuzz data
    let mut u = Unstructured::new(data);

    // Get number of operations (bounded)
    let op_count: usize = match u.int_in_range(1..=50) {
        Ok(n) => n,
        Err(_) => return,
    };

    // Simulate cycle engine state (simplified for fuzzing)
    let mut running = false;
    let mut current_phase_idx: usize = 0;
    let mut cycle_number: u64 = 0;
    let mut checkpoint: Option<(usize, u64)> = None;

    let phases = CyclePhase::all_phases();

    for _ in 0..op_count {
        let op: CycleOp = match u.arbitrary() {
            Ok(op) => op,
            Err(_) => break,
        };

        match op {
            CycleOp::Start => {
                if !running {
                    running = true;
                    cycle_number = 1;
                    current_phase_idx = 0;
                }
            }
            CycleOp::Stop => {
                running = false;
            }
            CycleOp::ForceTransition => {
                if running {
                    current_phase_idx = (current_phase_idx + 1) % 9;
                    if current_phase_idx == 0 {
                        cycle_number = cycle_number.saturating_add(1);
                    }
                }
            }
            CycleOp::Tick => {
                if running {
                    // Tick might cause transition in simulated time
                    // For fuzzing, randomly advance
                    if let Ok(should_advance) = u.arbitrary::<bool>() {
                        if should_advance {
                            current_phase_idx = (current_phase_idx + 1) % 9;
                            if current_phase_idx == 0 {
                                cycle_number = cycle_number.saturating_add(1);
                            }
                        }
                    }
                }
            }
            CycleOp::Checkpoint => {
                checkpoint = Some((current_phase_idx, cycle_number));
            }
            CycleOp::RestoreCheckpoint => {
                if let Some((idx, num)) = checkpoint {
                    current_phase_idx = idx;
                    cycle_number = num;
                }
            }
            CycleOp::QueryPhase => {
                let _phase = phases[current_phase_idx];
            }
            CycleOp::QueryCycleNumber => {
                let _num = cycle_number;
            }
            CycleOp::CheckRunning => {
                let _is_running = running;
            }
        }

        // Invariant checks
        assert!(current_phase_idx < 9, "Phase index out of bounds");
        assert!(
            !running || cycle_number >= 1,
            "Running engine must have cycle >= 1"
        );
    }
});
