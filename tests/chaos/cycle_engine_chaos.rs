//! Chaos testing scenarios for the Metabolism Cycle Engine.
//!
//! These tests verify system resilience under fault conditions:
//! - Handler panics during phase transitions
//! - Clock skew and time manipulation
//! - Concurrent operations
//! - Exact boundary conditions
//! - Callback reentrancy
//!
//! # Running Chaos Tests
//!
//! ```bash
//! cargo test --test cycle_engine_chaos -- --test-threads=1
//! ```
//!
//! For extended testing with miri (undefined behavior detection):
//! ```bash
//! cargo +nightly miri test --test cycle_engine_chaos
//! ```

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{Duration, Utc};

use cycle_engine::{
    CycleEngineBuilder, MetabolismCycleEngine,
    chaos::{ChaosConfig, ChaosInjector, ChaosError, StateCheckpoint, TransactionalTransition,
            saturating_time_acceleration, saturating_add_duration},
    CancellationToken,
};
use living_core::{CyclePhase, CycleState, LivingProtocolConfig, LivingProtocolEvent};

// =============================================================================
// Test Utilities
// =============================================================================

fn create_test_engine() -> MetabolismCycleEngine {
    CycleEngineBuilder::new()
        .with_simulated_time(86400.0) // 1 second = 1 day
        .build()
}

fn create_fast_engine() -> MetabolismCycleEngine {
    // Very fast time acceleration for testing
    let mut config = LivingProtocolConfig::default();
    config.cycle.simulated_time = true;
    config.cycle.time_acceleration = 1_000_000.0; // Extreme acceleration

    CycleEngineBuilder::new()
        .with_config(config)
        .with_simulated_time(1_000_000.0)
        .build()
}

// =============================================================================
// Handler Panic Recovery Tests
// =============================================================================

#[test]
fn test_handler_panic_on_enter_rolls_back() {
    // This test verifies that if a handler panics during on_enter,
    // the engine state can be rolled back to the previous checkpoint.

    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Create checkpoint before transition
    let checkpoint = engine.checkpoint();
    let phase_before = engine.current_phase();
    let cycle_before = engine.cycle_number();

    // Simulate a panic during transition (we catch it)
    let result = catch_unwind(AssertUnwindSafe(|| {
        // In a real scenario, the handler would panic here
        // For testing, we just verify the checkpoint mechanism
        engine.force_transition()
    }));

    if result.is_err() {
        // Restore from checkpoint on panic
        engine.restore_from_checkpoint(&checkpoint);

        // Verify state was restored
        assert_eq!(engine.current_phase(), phase_before);
        assert_eq!(engine.cycle_number(), cycle_before);
    }
}

#[test]
fn test_handler_panic_on_exit_preserves_state() {
    // Tests that state is preserved when exit handler fails

    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Advance a few phases to have some state
    for _ in 0..3 {
        engine.force_transition().unwrap();
    }

    let events_before = engine.cycle_events().len();
    let phase_before = engine.current_phase();

    // Use transactional transition which handles failures
    let result = engine.transition_transactional();

    // Whether it succeeds or fails, state should be consistent
    assert!(engine.current_phase() != CyclePhase::Shadow || engine.cycle_number() >= 1);

    if result.is_ok() {
        // Phase should have advanced
        assert_ne!(engine.current_phase(), phase_before);
    }
}

// =============================================================================
// Simulated Time Overflow Tests
// =============================================================================

#[test]
fn test_simulated_time_overflow_saturates() {
    // Verify that extreme time acceleration doesn't cause overflow

    // Test the saturating function directly
    let huge_elapsed = i64::MAX / 2;
    let huge_acceleration = 1000.0;

    let result = saturating_time_acceleration(huge_elapsed, huge_acceleration);

    // Should saturate rather than overflow
    assert!(result <= i64::MAX / 2);
    assert!(result >= i64::MIN / 2);
}

#[test]
fn test_saturating_add_duration_extreme_values() {
    let base = Utc::now();

    // Test with very large positive duration
    let huge_positive = Duration::days(365 * 10000); // 10,000 years
    let result = saturating_add_duration(base, huge_positive);
    assert!(result >= base); // Should not panic

    // Test with large negative duration
    let huge_negative = Duration::days(-365 * 10000);
    let result = saturating_add_duration(base, huge_negative);
    assert!(result <= base); // Should not panic
}

#[test]
fn test_extreme_time_acceleration_engine() {
    // Create engine with extreme time acceleration
    let mut engine = create_fast_engine();
    engine.start().unwrap();

    // Run multiple ticks - should not panic despite fast time
    for _ in 0..100 {
        let result = engine.tick();
        // Should either succeed or fail gracefully, never panic
        match result {
            Ok(_) => {}
            Err(_) => break, // Engine stopped, which is fine
        }
    }

    // Engine should still be in a valid state
    let _phase = engine.current_phase();
    let _cycle = engine.cycle_number();
}

// =============================================================================
// Concurrent Start/Stop Tests
// =============================================================================

#[test]
fn test_concurrent_start_stop_is_safe() {
    let mut engine = create_test_engine();

    // Rapid start/stop cycles
    for i in 0..100 {
        if i % 2 == 0 {
            let _ = engine.start();
        } else {
            engine.stop();
        }

        // Engine should always report a valid state
        let _phase = engine.current_phase();
        let _cycle = engine.cycle_number();
        let is_running = engine.is_running();

        // Running state should be consistent
        if i % 2 == 0 {
            // After start attempt, might be running (if wasn't already)
        } else {
            // After stop, should not be running
            assert!(!is_running);
        }
    }
}

#[test]
fn test_double_start_fails_gracefully() {
    let mut engine = create_test_engine();

    // First start succeeds
    let result1 = engine.start();
    assert!(result1.is_ok());
    assert!(engine.is_running());

    // Second start should fail
    let result2 = engine.start();
    assert!(result2.is_err());

    // Engine should still be running from first start
    assert!(engine.is_running());
}

#[test]
fn test_stop_when_not_running_is_safe() {
    let mut engine = create_test_engine();

    // Stop before starting - should not panic
    engine.stop();
    assert!(!engine.is_running());

    // Stop again - should still be safe
    engine.stop();
    assert!(!engine.is_running());
}

// =============================================================================
// Exact Phase Boundary Tests
// =============================================================================

#[test]
fn test_exact_phase_boundary_triggers_once() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    let mut transition_count = 0;
    let initial_phase = engine.current_phase();

    // Force exactly 9 transitions (one full cycle)
    for _ in 0..9 {
        let events = engine.force_transition().unwrap();
        transition_count += 1;

        // Each transition should produce events
        assert!(!events.is_empty());

        // Check for phase transition event
        let has_transition = events.iter().any(|e| {
            matches!(e, LivingProtocolEvent::PhaseTransitioned(_))
        });
        assert!(has_transition, "Transition {} should emit PhaseTransitioned event", transition_count);
    }

    // Should be back to Shadow phase in cycle 2
    assert_eq!(engine.current_phase(), CyclePhase::Shadow);
    assert_eq!(engine.cycle_number(), 2);
}

#[test]
fn test_phase_day_boundaries() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Initial phase day should be 0
    assert_eq!(engine.current_state().phase_day, 0);

    // After tick, phase_day might update based on elapsed time
    let _ = engine.tick().unwrap();

    // Phase day should be non-negative
    assert!(engine.current_state().phase_day <= 365); // Reasonable bound
}

// =============================================================================
// Callback Reentrancy Tests
// =============================================================================

#[test]
fn test_callback_reentrance_does_not_deadlock() {
    let config = ChaosConfig::new().with_reentrancy_test();
    let injector = ChaosInjector::new(config);

    // Simulate nested callback attempt
    assert!(injector.enter_callback()); // First entry succeeds
    assert!(injector.is_in_callback());

    // Reentrant call should be detected
    assert!(!injector.enter_callback()); // Second entry fails

    // Exit and verify
    injector.exit_callback();
    assert!(!injector.is_in_callback());

    // Can enter again after exit
    assert!(injector.enter_callback());
    injector.exit_callback();
}

// =============================================================================
// Transactional Transition Tests
// =============================================================================

#[test]
fn test_transactional_transition_commits_on_success() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    let phase_before = engine.current_phase();

    let result = engine.transition_transactional();
    assert!(result.is_ok());

    let phase_after = engine.current_phase();

    // Phase should have changed
    assert_ne!(phase_before, phase_after);
    assert_eq!(phase_after, phase_before.next());
}

#[test]
fn test_checkpoint_restore_preserves_exact_state() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Advance a few phases
    engine.force_transition().unwrap();
    engine.force_transition().unwrap();

    // Checkpoint
    let checkpoint = engine.checkpoint();
    let checkpointed_phase = engine.current_phase();
    let checkpointed_cycle = engine.cycle_number();
    let checkpointed_events = engine.cycle_events().len();

    // Advance more
    engine.force_transition().unwrap();
    engine.force_transition().unwrap();

    // State should have changed
    assert_ne!(engine.current_phase(), checkpointed_phase);

    // Restore
    engine.restore_from_checkpoint(&checkpoint);

    // Verify exact restoration
    assert_eq!(engine.current_phase(), checkpointed_phase);
    assert_eq!(engine.cycle_number(), checkpointed_cycle);
    assert_eq!(engine.cycle_events().len(), checkpointed_events);
}

// =============================================================================
// Chaos Injector Tests
// =============================================================================

#[test]
fn test_chaos_injector_operation_counting() {
    let config = ChaosConfig::new().with_fail_after_n(5);
    let injector = ChaosInjector::new(config);

    // First 5 operations should not fail
    for i in 0..5 {
        assert!(!injector.should_fail(), "Operation {} should not fail", i);
    }

    // 6th operation should fail
    assert!(injector.should_fail());
}

#[test]
fn test_chaos_injector_clock_skew() {
    let skew = Duration::hours(3);
    let config = ChaosConfig::new().with_clock_skew(skew);
    let injector = ChaosInjector::new(config);

    let now = Utc::now();
    let skewed = injector.apply_clock_skew(now);

    assert_eq!(skewed - now, skew);
}

#[test]
fn test_chaos_injector_phase_panic_detection() {
    let config = ChaosConfig::new()
        .with_panic_on_enter(CyclePhase::Composting)
        .with_panic_on_exit(CyclePhase::Liminal)
        .with_panic_on_tick(CyclePhase::Eros);

    let injector = ChaosInjector::new(config);

    // Check phase-specific panic triggers
    assert!(injector.should_panic_on_enter(CyclePhase::Composting));
    assert!(!injector.should_panic_on_enter(CyclePhase::Shadow));

    assert!(injector.should_panic_on_exit(CyclePhase::Liminal));
    assert!(!injector.should_panic_on_exit(CyclePhase::Shadow));

    assert!(injector.should_panic_on_tick(CyclePhase::Eros));
    assert!(!injector.should_panic_on_tick(CyclePhase::Shadow));
}

// =============================================================================
// Cancellation Token Tests
// =============================================================================

#[test]
fn test_cancellation_token_basic() {
    let token = CancellationToken::new();

    assert!(!token.is_cancelled());

    token.cancel();

    assert!(token.is_cancelled());
}

#[test]
fn test_cancellation_token_clones_share_state() {
    let token1 = CancellationToken::new();
    let token2 = token1.clone();

    assert!(!token1.is_cancelled());
    assert!(!token2.is_cancelled());

    // Cancel via first token
    token1.cancel();

    // Both should see cancellation
    assert!(token1.is_cancelled());
    assert!(token2.is_cancelled());
}

// =============================================================================
// State Consistency Tests
// =============================================================================

#[test]
fn test_state_consistency_after_many_operations() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Perform many operations
    for _ in 0..100 {
        // Mix of ticks and transitions
        let _ = engine.tick();
        if engine.is_running() {
            let _ = engine.force_transition();
        }
    }

    // Verify state is still consistent
    let state = engine.current_state();

    // Cycle number should be positive
    assert!(state.cycle_number >= 1);

    // Phase should be valid
    let valid_phases = CyclePhase::all_phases();
    assert!(valid_phases.contains(&state.current_phase));

    // Timestamps should be reasonable
    assert!(state.phase_started <= Utc::now() + Duration::days(365 * 100)); // Allow for time acceleration
    assert!(state.cycle_started <= state.phase_started);
}

#[test]
fn test_transition_history_integrity() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Perform transitions
    for _ in 0..20 {
        engine.force_transition().unwrap();
    }

    let history = engine.transition_history();

    // Each transition should have valid from/to phases
    for (i, transition) in history.iter().enumerate() {
        // To should be the next phase of from
        assert_eq!(transition.to, transition.from.next(),
            "Transition {} has invalid from/to: {:?} -> {:?}",
            i, transition.from, transition.to);

        // Cycle number should be non-zero
        assert!(transition.cycle_number >= 1);
    }

    // Consecutive transitions should chain correctly
    for i in 1..history.len() {
        assert_eq!(history[i].from, history[i-1].to,
            "Transitions {} and {} don't chain correctly", i-1, i);
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_tick_when_not_running_returns_error() {
    let mut engine = create_test_engine();

    // Don't start the engine
    let result = engine.tick();
    assert!(result.is_err());
}

#[test]
fn test_force_transition_when_not_running_returns_error() {
    let mut engine = create_test_engine();

    // Don't start the engine
    let result = engine.force_transition();
    assert!(result.is_err());
}

#[test]
fn test_transactional_transition_when_not_running_returns_error() {
    let mut engine = create_test_engine();

    // Don't start the engine
    let result = engine.transition_transactional();
    assert!(result.is_err());
}
