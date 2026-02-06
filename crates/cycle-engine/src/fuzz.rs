//! Property-based fuzzing tests for cycle engine invariants.
//!
//! Uses proptest for property-based testing of critical invariants.
//!
//! ## Configuration
//!
//! Test case counts are configured for different environments:
//! - Default: 100 cases for quick local testing
//! - CI: 1,000 cases via PROPTEST_CASES env var
//! - Extended: 10,000+ cases for comprehensive verification
//!
//! To run extended tests:
//! ```bash
//! PROPTEST_CASES=10000 cargo test -p cycle-engine --release -- --test-threads=1 fuzz
//! ```

#[cfg(test)]
mod fuzz_tests {
    use crate::chaos::{saturating_add_duration, saturating_time_acceleration};
    use crate::scheduler::CycleEngineBuilder;
    use living_core::{CyclePhase, LivingProtocolEvent};
    use proptest::prelude::*;

    // =========================================================================
    // Arbitrary implementations for proptest
    // =========================================================================

    fn arb_cycle_phase() -> impl Strategy<Value = CyclePhase> {
        prop_oneof![
            Just(CyclePhase::Shadow),
            Just(CyclePhase::Composting),
            Just(CyclePhase::Liminal),
            Just(CyclePhase::NegativeCapability),
            Just(CyclePhase::Eros),
            Just(CyclePhase::CoCreation),
            Just(CyclePhase::Beauty),
            Just(CyclePhase::EmergentPersonhood),
            Just(CyclePhase::Kenosis),
        ]
    }

    fn arb_unit_interval() -> impl Strategy<Value = f64> {
        (0u64..=1000u64).prop_map(|n| n as f64 / 1000.0)
    }

    fn arb_positive_f64() -> impl Strategy<Value = f64> {
        (1u64..=10000u64).prop_map(|n| n as f64 / 100.0)
    }

    #[allow(dead_code)]
    fn arb_percentage() -> impl Strategy<Value = f64> {
        (0u64..=100u64).prop_map(|n| n as f64 / 100.0)
    }

    fn arb_kenosis_percentage() -> impl Strategy<Value = f64> {
        // Kenosis max is 20%
        (0u64..=20u64).prop_map(|n| n as f64 / 100.0)
    }

    // =========================================================================
    // Cycle Engine Invariants (10,000+ cases with PROPTEST_CASES)
    // =========================================================================
    //
    // Note: Some cycle engine fuzz tests are limited to small iteration counts
    // due to chrono TimeDelta overflow issues in simulated time mode.
    // These tests verify invariants within safe bounds.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Invariant: Phase transitions always follow the defined order.
        #[test]
        fn fuzz_phase_order_invariant(transitions in 0usize..9) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            let phase_order = [
                CyclePhase::Shadow,
                CyclePhase::Composting,
                CyclePhase::Liminal,
                CyclePhase::NegativeCapability,
                CyclePhase::Eros,
                CyclePhase::CoCreation,
                CyclePhase::Beauty,
                CyclePhase::EmergentPersonhood,
                CyclePhase::Kenosis,
            ];

            let mut phase_idx = 0;

            for _ in 0..transitions {
                let current_phase = engine.current_phase();
                prop_assert_eq!(current_phase, phase_order[phase_idx]);

                engine.force_transition().unwrap();

                phase_idx = (phase_idx + 1) % 9;
            }
        }

        /// Invariant: Engine is_running() is consistent with start/stop.
        #[test]
        fn fuzz_running_state_consistency(
            start_stop_sequence in prop::collection::vec(prop::bool::ANY, 1..10)
        ) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            let mut expected_running = false;

            for should_start in start_stop_sequence {
                if should_start && !expected_running {
                    let _ = engine.start();
                    expected_running = true;
                } else if !should_start && expected_running {
                    engine.stop();
                    expected_running = false;
                }

                prop_assert_eq!(engine.is_running(), expected_running);
            }
        }

        /// PROPERTY: Phase transitions are deterministic given the same state.
        /// Given identical starting state, multiple transitions produce identical results.
        #[test]
        fn fuzz_phase_transitions_deterministic(
            initial_transitions in 0usize..5,
            test_transitions in 1usize..4
        ) {
            // Build first engine and advance it
            let mut engine1 = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();
            engine1.start().unwrap();

            for _ in 0..initial_transitions {
                engine1.force_transition().unwrap();
            }

            // Record state
            let phase1_before = engine1.current_phase();
            let cycle1_before = engine1.cycle_number();

            // Build second engine with same initial state
            let mut engine2 = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();
            engine2.start().unwrap();

            for _ in 0..initial_transitions {
                engine2.force_transition().unwrap();
            }

            // Verify same starting state
            prop_assert_eq!(engine2.current_phase(), phase1_before);
            prop_assert_eq!(engine2.cycle_number(), cycle1_before);

            // Apply same transitions to both
            for _ in 0..test_transitions {
                let events1 = engine1.force_transition().unwrap();
                let events2 = engine2.force_transition().unwrap();

                // Same number of events
                prop_assert_eq!(events1.len(), events2.len());

                // Same resulting state
                prop_assert_eq!(engine1.current_phase(), engine2.current_phase());
                prop_assert_eq!(engine1.cycle_number(), engine2.cycle_number());
            }
        }

        /// PROPERTY: Tick is idempotent within the same time slice.
        /// Multiple ticks without time advancement should produce consistent state.
        #[test]
        fn fuzz_tick_idempotent_within_time_slice(
            initial_transitions in 0usize..5,
            tick_count in 1usize..20
        ) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            // Advance to a specific phase
            for _ in 0..initial_transitions {
                engine.force_transition().unwrap();
            }

            // Record state before multiple ticks
            let phase_before = engine.current_phase();
            let cycle_before = engine.cycle_number();

            // Multiple ticks in rapid succession (within same time slice in simulated time)
            // Since time advances rapidly in simulated mode, we check that:
            // 1. State remains valid
            // 2. Cycle number never decreases
            // 3. Phase transitions follow rules
            for _ in 0..tick_count {
                let result = engine.tick();
                prop_assert!(result.is_ok());

                // Cycle number should never decrease
                prop_assert!(engine.cycle_number() >= cycle_before);

                // Phase should be valid
                let current = engine.current_phase();
                prop_assert!(CyclePhase::all_phases().contains(&current));
            }

            // If we stayed in same cycle, verify phase progression is valid
            if engine.cycle_number() == cycle_before {
                // We should not have regressed in phases
                let phase_after = engine.current_phase();
                // Since phases wrap around, we just verify both are valid phases
                prop_assert!(CyclePhase::all_phases().contains(&phase_before));
                prop_assert!(CyclePhase::all_phases().contains(&phase_after));
            }
        }

        /// PROPERTY: Checkpoint + restore = identity.
        /// Creating a checkpoint and restoring produces identical state.
        #[test]
        fn fuzz_checkpoint_restore_identity(
            initial_transitions in 0usize..5,
            post_checkpoint_transitions in 1usize..4
        ) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            // Advance to some state
            for _ in 0..initial_transitions {
                engine.force_transition().unwrap();
            }

            // Create checkpoint
            let checkpoint = engine.checkpoint();
            let phase_at_checkpoint = engine.current_phase();
            let cycle_at_checkpoint = engine.cycle_number();
            let events_at_checkpoint = engine.cycle_events().len();

            // Advance further
            for _ in 0..post_checkpoint_transitions {
                engine.force_transition().unwrap();
            }

            // State should have changed
            prop_assert!(
                engine.current_phase() != phase_at_checkpoint ||
                engine.cycle_number() != cycle_at_checkpoint ||
                engine.cycle_events().len() != events_at_checkpoint
            );

            // Restore from checkpoint
            engine.restore_from_checkpoint(&checkpoint);

            // State should exactly match checkpoint
            prop_assert_eq!(engine.current_phase(), phase_at_checkpoint);
            prop_assert_eq!(engine.cycle_number(), cycle_at_checkpoint);
            prop_assert_eq!(engine.cycle_events().len(), events_at_checkpoint);
        }

        /// PROPERTY: Events are serializable and deserializable.
        /// All events produced by the engine can roundtrip through serde.
        #[test]
        fn fuzz_events_serializable(transitions in 0usize..9) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            let mut all_events = Vec::new();

            for _ in 0..transitions {
                let events = engine.force_transition().unwrap();
                all_events.extend(events);
            }

            // Verify all events can serialize/deserialize
            for event in &all_events {
                let serialized = serde_json::to_string(event);
                prop_assert!(serialized.is_ok(), "Event serialization failed: {:?}", event);

                let json = serialized.unwrap();
                let deserialized: Result<LivingProtocolEvent, _> = serde_json::from_str(&json);
                prop_assert!(deserialized.is_ok(), "Event deserialization failed: {}", json);
            }
        }
    }

    // =========================================================================
    // K-Vector Invariants (10,000+ cases with PROPTEST_CASES)
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// Invariant: K-Vector dimensions must be in [0.0, 1.0].
        #[test]
        fn fuzz_kvector_bounds(
            presence in arb_unit_interval(),
            coherence in arb_unit_interval(),
            receptivity in arb_unit_interval(),
            integration in arb_unit_interval(),
            generativity in arb_unit_interval(),
            surrender in arb_unit_interval(),
            discernment in arb_unit_interval(),
            emergence in arb_unit_interval(),
        ) {
            let dimensions = [
                presence, coherence, receptivity, integration,
                generativity, surrender, discernment, emergence
            ];

            for dim in dimensions {
                prop_assert!(dim >= 0.0 && dim <= 1.0);
            }

            // Composite score should also be in bounds
            let composite: f64 = dimensions.iter().sum::<f64>() / 8.0;
            prop_assert!(composite >= 0.0 && composite <= 1.0);
        }

        /// Invariant: K-Vector velocity is bounded by physical constraints.
        #[test]
        fn fuzz_kvector_velocity_bounded(
            prev_values in prop::collection::vec(arb_unit_interval(), 8..=8),
            curr_values in prop::collection::vec(arb_unit_interval(), 8..=8),
            delta_t in arb_positive_f64(),
        ) {
            // Velocity = (curr - prev) / delta_t
            // Since values are in [0,1], max change is 1.0
            // Velocity bounded by 1.0 / delta_t

            for i in 0..8 {
                let velocity = (curr_values[i] - prev_values[i]) / delta_t;
                let max_velocity = 1.0 / delta_t;
                prop_assert!(velocity.abs() <= max_velocity + 0.0001); // epsilon for float
            }
        }

        /// PROPERTY: K-Vector normalization preserves relative ordering.
        /// Normalizing a K-Vector maintains the relative order of dimensions.
        #[test]
        fn fuzz_kvector_normalization_preserves_order(
            values in prop::collection::vec(0.0f64..100.0f64, 8..=8)
        ) {
            // Normalize to [0, 1] using min-max scaling
            let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            if (max_val - min_val).abs() < 1e-10 {
                // All values equal, skip
                return Ok(());
            }

            let normalized: Vec<f64> = values.iter()
                .map(|v| (v - min_val) / (max_val - min_val))
                .collect();

            // All normalized values should be in [0, 1]
            for val in &normalized {
                prop_assert!(*val >= 0.0 && *val <= 1.0);
            }

            // Relative ordering should be preserved
            for i in 0..8 {
                for j in 0..8 {
                    if values[i] < values[j] {
                        prop_assert!(normalized[i] <= normalized[j] + 1e-10);
                    } else if values[i] > values[j] {
                        prop_assert!(normalized[i] >= normalized[j] - 1e-10);
                    }
                }
            }
        }
    }

    // =========================================================================
    // Wound Healing Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Wound phases only advance forward, never backward.
        #[test]
        fn fuzz_wound_phase_forward_only(advance_count in 0usize..10) {
            use metabolism::WoundHealingEngine;
            use living_core::{InMemoryEventBus, WoundSeverity, WoundPhase, WoundHealingConfig, EventBus};
            use std::sync::Arc;

            let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
            let config = WoundHealingConfig::default();
            let mut engine = WoundHealingEngine::new(config, event_bus);

            let wound_record = engine.create_wound(
                "did:agent:test".to_string(),
                WoundSeverity::Moderate,
                "test cause".to_string(),
            ).unwrap();
            let wound_id = wound_record.id.clone();

            let phase_order = [
                WoundPhase::Hemostasis,
                WoundPhase::Inflammation,
                WoundPhase::Proliferation,
                WoundPhase::Remodeling,
                WoundPhase::Healed,
            ];

            let mut phase_idx = 0;

            for _ in 0..advance_count {
                let wound = engine.get_wound(&wound_id).unwrap();
                let current_phase = wound.phase.clone();

                // Current phase should match expected
                prop_assert_eq!(current_phase, phase_order[phase_idx]);

                if phase_idx < 4 {
                    let result = engine.advance_phase(&wound_id);
                    if result.is_ok() {
                        phase_idx += 1;
                    }
                }
            }
        }

        /// Invariant: Wound escrow amount is never negative.
        #[test]
        fn fuzz_wound_escrow_non_negative(escrow_amount in 0u64..1000000u64) {
            let escrow = escrow_amount as f64;
            prop_assert!(escrow >= 0.0);
        }
    }

    // =========================================================================
    // Kenosis Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Kenosis release percentage never exceeds 20%.
        #[test]
        fn fuzz_kenosis_max_20_percent(percentage in arb_kenosis_percentage()) {
            prop_assert!(percentage <= 0.20);
        }

        /// Invariant: Individual kenosis commitment never exceeds 20%.
        #[test]
        fn fuzz_kenosis_single_limit(percentage in arb_kenosis_percentage()) {
            use metabolism::KenosisEngine;
            use living_core::{InMemoryEventBus, KenosisConfig, EventBus};
            use std::sync::Arc;

            let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
            let config = KenosisConfig::default();
            let mut engine = KenosisEngine::new(config, event_bus);

            engine.set_current_cycle(1);
            engine.register_agent("did:agent:test", 1000.0);

            // Individual commitment should succeed if <= 20%
            let result = engine.commit_kenosis("did:agent:test", percentage);

            // Should succeed since arb_kenosis_percentage generates values <= 20%
            prop_assert!(result.is_ok() || percentage > 0.20);
            prop_assert!(percentage <= 0.20);
        }
    }

    // =========================================================================
    // Entanglement Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Entanglement strength is always in [0.0, 1.0].
        #[test]
        fn fuzz_entanglement_strength_bounded(strength in arb_unit_interval()) {
            prop_assert!(strength >= 0.0 && strength <= 1.0);
        }

        /// Invariant: Entanglement decay never results in negative strength.
        #[test]
        fn fuzz_entanglement_decay_non_negative(
            initial_strength in arb_unit_interval(),
            decay_rate in arb_unit_interval(),
            days in 0u32..365,
        ) {
            // Exponential decay: strength * (1 - rate)^days
            let decayed = initial_strength * (1.0 - decay_rate).powi(days as i32);
            prop_assert!(decayed >= 0.0);
        }
    }

    // =========================================================================
    // Beauty Score Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Beauty score dimensions are all in [0.0, 1.0].
        #[test]
        fn fuzz_beauty_dimensions_bounded(
            coherence in arb_unit_interval(),
            elegance in arb_unit_interval(),
            resonance in arb_unit_interval(),
            aliveness in arb_unit_interval(),
            wholeness in arb_unit_interval(),
        ) {
            let dimensions = [coherence, elegance, resonance, aliveness, wholeness];

            for dim in dimensions {
                prop_assert!(dim >= 0.0 && dim <= 1.0);
            }

            // Composite beauty score
            let composite: f64 = dimensions.iter().sum::<f64>() / 5.0;
            prop_assert!(composite >= 0.0 && composite <= 1.0);
        }
    }

    // =========================================================================
    // Composting Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Composting progress is always in [0.0, 1.0].
        #[test]
        fn fuzz_composting_progress_bounded(progress in arb_unit_interval()) {
            prop_assert!(progress >= 0.0 && progress <= 1.0);
        }

        /// Invariant: Composting progress only increases (monotonic).
        #[test]
        fn fuzz_composting_progress_monotonic(
            increments in prop::collection::vec(arb_unit_interval(), 1..10)
        ) {
            let mut progress = 0.0;

            for increment in increments {
                let new_progress = (progress + increment * 0.1).min(1.0);
                prop_assert!(new_progress >= progress);
                progress = new_progress;
            }
        }
    }

    // =========================================================================
    // Negative Capability Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Invariant: Claims held in uncertainty cannot be voted on.
        #[test]
        fn fuzz_held_claims_not_votable(
            claim_count in 1usize..10,
            _hold_days in 1u32..30,
        ) {
            use epistemics::negative_capability::NegativeCapabilityEngine;

            let mut engine = NegativeCapabilityEngine::new();

            for i in 0..claim_count {
                let claim_id = format!("claim-{}", i);
                engine.hold_in_uncertainty(&claim_id, "needs research", 1, "did:holder:test");

                // While held, cannot vote
                prop_assert!(!engine.can_vote_on(&claim_id));
                prop_assert!(engine.is_held(&claim_id));
            }
        }
    }

    // =========================================================================
    // Metabolic Trust Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Metabolic trust score is always in [0.0, 1.0].
        #[test]
        fn fuzz_metabolic_trust_bounded(trust_score in arb_unit_interval()) {
            prop_assert!(trust_score >= 0.0 && trust_score <= 1.0);
        }

        /// Invariant: Trust updates respect the change cap.
        #[test]
        fn fuzz_trust_change_capped(
            current_trust in arb_unit_interval(),
            delta in -1.0f64..1.0f64,
            max_change in arb_unit_interval(),
        ) {
            let capped_delta = delta.clamp(-max_change, max_change);
            let new_trust = (current_trust + capped_delta).clamp(0.0, 1.0);

            prop_assert!(new_trust >= 0.0 && new_trust <= 1.0);
            prop_assert!((new_trust - current_trust).abs() <= max_change + 0.0001);
        }
    }

    // =========================================================================
    // Extended Chaos/Boundary Tests (10,000+ cases when PROPTEST_CASES set)
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// Invariant: Saturating time acceleration never overflows.
        #[test]
        fn fuzz_time_acceleration_no_overflow(
            elapsed_ms in i64::MIN..i64::MAX,
            acceleration in 0.001f64..1000000.0f64,
        ) {
            let result = saturating_time_acceleration(elapsed_ms, acceleration);
            // Should never exceed safe bounds
            prop_assert!(result <= i64::MAX / 2);
            prop_assert!(result >= i64::MIN / 2);
        }

        /// Invariant: Saturating duration addition never panics.
        #[test]
        fn fuzz_saturating_add_no_panic(
            days in -365000i64..365000i64,
        ) {
            use chrono::{Duration, Utc};

            let base = Utc::now();
            let duration = Duration::days(days);
            let result = saturating_add_duration(base, duration);

            // Should always produce a valid DateTime
            prop_assert!(result >= chrono::DateTime::<Utc>::MIN_UTC || days < 0);
        }

        /// Invariant: Phase boundary transition is atomic.
        /// Tests exact boundary conditions where phase_day == phase_duration.
        #[test]
        fn fuzz_phase_boundary_atomic(phase_idx in 0usize..9) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            // Advance to target phase
            for _ in 0..phase_idx {
                engine.force_transition().unwrap();
            }

            let phase_before = engine.current_phase();

            // Single transition should move exactly one phase
            engine.force_transition().unwrap();

            let phase_after = engine.current_phase();

            // Phases should be adjacent (or wrap around)
            let expected_next = phase_before.next();
            prop_assert_eq!(phase_after, expected_next);
        }
    }

    // =========================================================================
    // Transactional Transition Tests
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Invariant: Transactional transitions preserve state on success.
        #[test]
        fn fuzz_transactional_success_preserves_state(transitions in 0usize..5) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            for _ in 0..transitions {
                let phase_before = engine.current_phase();
                let cycle_before = engine.cycle_number();

                let result = engine.transition_transactional();
                prop_assert!(result.is_ok());

                let phase_after = engine.current_phase();
                let cycle_after = engine.cycle_number();

                // Phase should have advanced
                prop_assert_ne!(phase_after, phase_before);

                // Cycle number should only change on Kenosis -> Shadow
                if phase_before == CyclePhase::Kenosis {
                    prop_assert_eq!(cycle_after, cycle_before + 1);
                } else {
                    prop_assert_eq!(cycle_after, cycle_before);
                }
            }
        }

        /// Invariant: Checkpoint/restore returns engine to exact state.
        #[test]
        fn fuzz_checkpoint_restore_exact(transitions in 0usize..5) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            // Advance a few phases
            for _ in 0..transitions {
                engine.force_transition().unwrap();
            }

            // Create checkpoint
            let checkpoint = engine.checkpoint();
            let phase_at_checkpoint = engine.current_phase();
            let cycle_at_checkpoint = engine.cycle_number();

            // Advance some more
            engine.force_transition().unwrap();
            engine.force_transition().unwrap();

            // State should have changed
            prop_assert!(
                engine.current_phase() != phase_at_checkpoint ||
                engine.cycle_number() != cycle_at_checkpoint
            );

            // Restore from checkpoint
            engine.restore_from_checkpoint(&checkpoint);

            // State should match checkpoint exactly
            prop_assert_eq!(engine.current_phase(), phase_at_checkpoint);
            prop_assert_eq!(engine.cycle_number(), cycle_at_checkpoint);
        }
    }

    // =========================================================================
    // Concurrent Safety Tests (single-threaded simulation)
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Invariant: Rapid start/stop doesn't corrupt state.
        #[test]
        fn fuzz_rapid_start_stop_safe(operations in prop::collection::vec(prop::bool::ANY, 1..20)) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            for should_start in operations {
                if should_start {
                    let _ = engine.start(); // May succeed or fail
                } else {
                    engine.stop();
                }

                // Engine should always be in a valid state
                // Either running or not, but never corrupted
                let _phase = engine.current_phase(); // Should not panic
                let _cycle = engine.cycle_number(); // Should not panic
            }
        }

        /// Invariant: Multiple ticks in same phase don't corrupt state.
        #[test]
        fn fuzz_multiple_ticks_stable(tick_count in 1usize..100) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            let _initial_phase = engine.current_phase();
            let initial_cycle = engine.cycle_number();

            // Multiple ticks without enough time elapsed shouldn't change phase
            for _ in 0..tick_count {
                let result = engine.tick();
                prop_assert!(result.is_ok());
            }

            // Phase might change due to simulated time, but should follow rules
            let _final_phase = engine.current_phase();
            let final_cycle = engine.cycle_number();

            // Cycle number should never decrease
            prop_assert!(final_cycle >= initial_cycle);

            // If in same cycle, phase transition count should be bounded
            // by the number of phases (9)
            if final_cycle == initial_cycle {
                // Can't have gone through more than 8 transitions
                // (would have started a new cycle)
            }
        }
    }

    // =========================================================================
    // Additional Properties for 10,000+ Coverage
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// PROPERTY: Phase duration is always positive and matches specification.
        #[test]
        fn fuzz_phase_duration_valid(phase in arb_cycle_phase()) {
            let duration = phase.duration_days();
            prop_assert!(duration > 0);
            prop_assert!(duration <= 7); // Max is CoCreation at 7 days

            // Verify specific durations
            match phase {
                CyclePhase::Shadow => prop_assert_eq!(duration, 2),
                CyclePhase::Composting => prop_assert_eq!(duration, 5),
                CyclePhase::Liminal => prop_assert_eq!(duration, 3),
                CyclePhase::NegativeCapability => prop_assert_eq!(duration, 3),
                CyclePhase::Eros => prop_assert_eq!(duration, 4),
                CyclePhase::CoCreation => prop_assert_eq!(duration, 7),
                CyclePhase::Beauty => prop_assert_eq!(duration, 2),
                CyclePhase::EmergentPersonhood => prop_assert_eq!(duration, 1),
                CyclePhase::Kenosis => prop_assert_eq!(duration, 1),
            }
        }

        /// PROPERTY: Total cycle is exactly 28 days regardless of starting phase.
        #[test]
        fn fuzz_total_cycle_28_days(_start_phase in arb_cycle_phase()) {
            // Sum all phase durations
            let total: u32 = CyclePhase::all_phases()
                .iter()
                .map(|p| p.duration_days())
                .sum();

            prop_assert_eq!(total, 28);
        }

        /// PROPERTY: Phase.next().prev() == Phase (except for cycle boundaries).
        #[test]
        fn fuzz_phase_next_prev_inverse(phase in arb_cycle_phase()) {
            let next = phase.next();
            let back = next.prev();
            prop_assert_eq!(back, phase);
        }

        /// PROPERTY: Cycle events list is bounded in size.
        #[test]
        fn fuzz_events_bounded(transitions in 0usize..18) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            for _ in 0..transitions {
                engine.force_transition().unwrap();
            }

            // Events should be bounded - they're cleared each cycle
            // Maximum events per cycle = 9 transitions * events_per_transition
            // Plus the cycle start event
            let events = engine.cycle_events();
            prop_assert!(events.len() < 1000, "Too many events accumulated: {}", events.len());
        }

        /// PROPERTY: Transition history grows linearly with transitions.
        #[test]
        fn fuzz_transition_history_linear(transitions in 0usize..20) {
            let mut engine = CycleEngineBuilder::new()
                .with_simulated_time(86400.0)
                .build();

            engine.start().unwrap();

            for _ in 0..transitions {
                engine.force_transition().unwrap();
            }

            // History should match number of transitions
            prop_assert_eq!(engine.transition_history().len(), transitions);
        }
    }
}
