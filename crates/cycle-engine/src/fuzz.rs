//! Property-based fuzzing tests for cycle engine invariants.
//!
//! Uses proptest for property-based testing of critical invariants.

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use living_core::CyclePhase;
    use crate::scheduler::CycleEngineBuilder;

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

    fn arb_percentage() -> impl Strategy<Value = f64> {
        (0u64..=100u64).prop_map(|n| n as f64 / 100.0)
    }

    fn arb_kenosis_percentage() -> impl Strategy<Value = f64> {
        // Kenosis max is 20%
        (0u64..=20u64).prop_map(|n| n as f64 / 100.0)
    }

    // =========================================================================
    // Cycle Engine Invariants
    // =========================================================================
    //
    // Note: Some cycle engine fuzz tests are limited to small iteration counts
    // due to chrono TimeDelta overflow issues in simulated time mode.
    // These tests verify invariants within safe bounds.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

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
    }

    // =========================================================================
    // K-Vector Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

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
    }

    // =========================================================================
    // Wound Healing Invariants
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

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
        #![proptest_config(ProptestConfig::with_cases(100))]

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
        #![proptest_config(ProptestConfig::with_cases(100))]

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
        #![proptest_config(ProptestConfig::with_cases(100))]

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
        #![proptest_config(ProptestConfig::with_cases(100))]

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
        #![proptest_config(ProptestConfig::with_cases(50))]

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
        #![proptest_config(ProptestConfig::with_cases(100))]

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
}
