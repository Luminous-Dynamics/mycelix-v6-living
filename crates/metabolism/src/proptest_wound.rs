//! Property-based tests for wound healing phase transitions.
//!
//! This module provides comprehensive property-based testing for the wound
//! healing state machine, ensuring that all invariants are maintained.
//!
//! ## Key Properties Tested
//!
//! 1. Phase transitions are forward-only (no regression)
//! 2. Scar tissue strength is always > 1.0
//! 3. Restitution amounts are never negative
//! 4. Gate 1 invariants always pass for valid state
//!
//! ## Running Extended Tests
//!
//! ```bash
//! PROPTEST_CASES=10000 cargo test -p metabolism --release -- proptest
//! ```

#[cfg(test)]
mod proptest_wound_tests {
    use crate::wound_healing::WoundHealingEngine;
    use living_core::{InMemoryEventBus, WoundHealingConfig, WoundPhase, WoundSeverity};
    use living_core::traits::LivingPrimitive;
    use proptest::prelude::*;
    use std::sync::Arc;

    // =========================================================================
    // Arbitrary Implementations
    // =========================================================================

    fn arb_wound_severity() -> impl Strategy<Value = WoundSeverity> {
        prop_oneof![
            Just(WoundSeverity::Minor),
            Just(WoundSeverity::Moderate),
            Just(WoundSeverity::Severe),
            Just(WoundSeverity::Critical),
        ]
    }

    fn arb_wound_phase() -> impl Strategy<Value = WoundPhase> {
        prop_oneof![
            Just(WoundPhase::Hemostasis),
            Just(WoundPhase::Inflammation),
            Just(WoundPhase::Proliferation),
            Just(WoundPhase::Remodeling),
            Just(WoundPhase::Healed),
        ]
    }

    fn arb_unit_interval() -> impl Strategy<Value = f64> {
        (0u64..=1000u64).prop_map(|n| n as f64 / 1000.0)
    }

    fn arb_positive_amount() -> impl Strategy<Value = f64> {
        (0u64..=1000000u64).prop_map(|n| n as f64 / 100.0)
    }

    fn make_engine() -> WoundHealingEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        WoundHealingEngine::new(WoundHealingConfig::default(), bus)
    }

    // =========================================================================
    // Forward-Only Phase Transition Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// PROPERTY: Phase transitions are strictly forward-only.
        /// No wound can ever regress to a previous phase.
        #[test]
        fn prop_phase_transitions_forward_only(
            severity in arb_wound_severity(),
            advance_count in 0usize..20,
        ) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                format!("did:agent:proptest-{}", advance_count),
                severity,
                "proptest cause".to_string(),
            ).unwrap();

            let wound_id = wound.id.clone();
            let mut prev_phase_idx = 0usize;

            let phase_order = [
                WoundPhase::Hemostasis,
                WoundPhase::Inflammation,
                WoundPhase::Proliferation,
                WoundPhase::Remodeling,
                WoundPhase::Healed,
            ];

            for _ in 0..advance_count {
                let current_phase = engine.get_wound(&wound_id).unwrap().phase;

                // Find current phase index
                let current_idx = phase_order.iter()
                    .position(|p| *p == current_phase)
                    .unwrap();

                // Current phase index must be >= previous
                prop_assert!(
                    current_idx >= prev_phase_idx,
                    "Phase regressed from {:?} (idx {}) to {:?} (idx {})",
                    phase_order[prev_phase_idx],
                    prev_phase_idx,
                    current_phase,
                    current_idx
                );

                // Try to advance
                if current_phase != WoundPhase::Healed {
                    let _ = engine.advance_phase(&wound_id);
                }

                prev_phase_idx = current_idx;
            }
        }

        /// PROPERTY: Cannot advance past Healed state.
        /// Once healed, advancing returns an error.
        #[test]
        fn prop_cannot_advance_past_healed(severity in arb_wound_severity()) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:healed-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();
            let wound_id = wound.id.clone();

            // Advance to Healed
            for _ in 0..4 {
                let _ = engine.advance_phase(&wound_id);
            }

            let current_phase = engine.get_wound(&wound_id).unwrap().phase;
            prop_assert_eq!(current_phase, WoundPhase::Healed);

            // Trying to advance should fail
            let result = engine.advance_phase(&wound_id);
            prop_assert!(result.is_err());
        }

        /// PROPERTY: Phase history is strictly monotonic.
        /// Each entry in phase_history must be strictly greater than the previous.
        #[test]
        fn prop_phase_history_monotonic(
            severity in arb_wound_severity(),
            advance_count in 0usize..10,
        ) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:history-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            for _ in 0..advance_count {
                let _ = engine.advance_phase(&wound.id);
            }

            let wound = engine.get_wound(&wound.id).unwrap();
            let history = &wound.phase_history;

            // Verify monotonic increase in phases
            for window in history.windows(2) {
                let from = &window[0].0;
                let to = &window[1].0;

                prop_assert!(
                    from.can_transition_to(to),
                    "Invalid transition in history: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }

    // =========================================================================
    // Scar Tissue Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: Scar tissue strength is always > 1.0.
        /// Healed wounds are stronger than before.
        #[test]
        fn prop_scar_tissue_strength_gt_one(severity in arb_wound_severity()) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:scar-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            // Advance to Remodeling
            for _ in 0..3 {
                engine.advance_phase(&wound.id).unwrap();
            }

            // Form scar tissue
            let scar = engine.form_scar_tissue(&wound.id);
            if let Ok(scar) = scar {
                prop_assert!(
                    scar.strength_multiplier > 1.0,
                    "Scar tissue strength {} is not > 1.0",
                    scar.strength_multiplier
                );
            }
        }

        /// PROPERTY: Scar tissue strength varies by severity.
        /// More severe wounds produce stronger scar tissue.
        #[test]
        fn prop_scar_strength_increases_with_severity(_dummy in 0..1) {
            let mut engine = make_engine();

            let mut scar_strengths = Vec::new();

            for severity in [
                WoundSeverity::Minor,
                WoundSeverity::Moderate,
                WoundSeverity::Severe,
                WoundSeverity::Critical,
            ] {
                let wound = engine.create_wound(
                    format!("did:agent:{:?}", severity),
                    severity,
                    "test".to_string(),
                ).unwrap();

                // Advance to Remodeling
                for _ in 0..3 {
                    engine.advance_phase(&wound.id).unwrap();
                }

                let scar = engine.form_scar_tissue(&wound.id).unwrap();
                scar_strengths.push(scar.strength_multiplier);
            }

            // Verify increasing order (or at least non-decreasing)
            for i in 1..scar_strengths.len() {
                prop_assert!(
                    scar_strengths[i] >= scar_strengths[i-1],
                    "Scar strength did not increase with severity: {:?}",
                    scar_strengths
                );
            }
        }

        /// PROPERTY: Cannot form scar tissue before Remodeling phase.
        #[test]
        fn prop_no_scar_before_remodeling(
            severity in arb_wound_severity(),
            early_advance_count in 0usize..3,
        ) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:early-scar-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            let wound_id = wound.id.clone();

            // Advance less than 3 times (before Remodeling)
            for _ in 0..early_advance_count.min(2) {
                let _ = engine.advance_phase(&wound_id);
            }

            let current_phase = engine.get_wound(&wound_id).unwrap().phase;
            if current_phase != WoundPhase::Remodeling && current_phase != WoundPhase::Healed {
                let result = engine.form_scar_tissue(&wound_id);
                prop_assert!(result.is_err(), "Should not form scar in {:?} phase", current_phase);
            }
        }
    }

    // =========================================================================
    // Restitution Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: Restitution amount is never negative.
        #[test]
        fn prop_restitution_non_negative(severity in arb_wound_severity()) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:restitution-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            if let Some(ref restitution) = wound.restitution_required {
                if let Some(amount) = restitution.amount_flow {
                    prop_assert!(
                        amount >= 0.0,
                        "Restitution amount {} is negative",
                        amount
                    );
                }
            }
        }

        /// PROPERTY: Restitution is generated for all wounds.
        #[test]
        fn prop_restitution_always_present(severity in arb_wound_severity()) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:restitution-present-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            prop_assert!(
                wound.restitution_required.is_some(),
                "Wound should have restitution requirement"
            );
        }
    }

    // =========================================================================
    // Gate 1 Invariant Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: Gate 1 checks always pass for valid engine state.
        #[test]
        fn prop_gate1_always_passes(
            wound_count in 1usize..5,
            advances_each in 0usize..5,
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let mut engine = WoundHealingEngine::new(WoundHealingConfig::default(), bus);

            // Create multiple wounds
            for i in 0..wound_count {
                let wound = engine.create_wound(
                    format!("did:agent:{}", i),
                    WoundSeverity::Moderate,
                    format!("cause-{}", i),
                ).unwrap();

                // Advance each wound
                for _ in 0..advances_each {
                    let _ = engine.advance_phase(&wound.id);
                }

                // Try to form scar tissue if possible
                let _ = engine.form_scar_tissue(&wound.id);
            }

            // Gate 1 should always pass
            let checks = engine.gate1_check();
            for check in &checks {
                prop_assert!(
                    check.passed,
                    "Gate 1 failed: {} - {:?}",
                    check.invariant,
                    check.details
                );
            }
        }

        /// PROPERTY: Phase count never exceeds 5 (Hemostasis -> Healed).
        #[test]
        fn prop_phase_count_bounded(
            severity in arb_wound_severity(),
            advance_attempts in 0usize..20,
        ) {
            let mut engine = make_engine();

            let wound = engine.create_wound(
                "did:agent:phase-count-test".to_string(),
                severity,
                "test".to_string(),
            ).unwrap();

            let mut successful_advances = 0;

            for _ in 0..advance_attempts {
                if engine.advance_phase(&wound.id).is_ok() {
                    successful_advances += 1;
                }
            }

            // Maximum 4 successful advances (Hemostasis -> ... -> Healed)
            prop_assert!(
                successful_advances <= 4,
                "Too many successful advances: {}",
                successful_advances
            );
        }
    }

    // =========================================================================
    // Multi-Agent Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// PROPERTY: Wounds for different agents are independent.
        #[test]
        fn prop_wounds_independent(
            agent_count in 2usize..5,
            advances in prop::collection::vec(0usize..5, 2..5),
        ) {
            let mut engine = make_engine();

            let mut wound_ids = Vec::new();

            // Create wounds for different agents
            for i in 0..agent_count {
                let wound = engine.create_wound(
                    format!("did:agent:{}", i),
                    WoundSeverity::Moderate,
                    "test".to_string(),
                ).unwrap();
                wound_ids.push(wound.id.clone());
            }

            // Advance each wound differently
            for (i, &advance_count) in advances.iter().enumerate().take(agent_count) {
                for _ in 0..advance_count {
                    let _ = engine.advance_phase(&wound_ids[i]);
                }
            }

            // Verify each wound has independent state
            for (i, &advance_count) in advances.iter().enumerate().take(agent_count) {
                let wound = engine.get_wound(&wound_ids[i]).unwrap();
                let expected_phase_idx = advance_count.min(4);

                let phases = [
                    WoundPhase::Hemostasis,
                    WoundPhase::Inflammation,
                    WoundPhase::Proliferation,
                    WoundPhase::Remodeling,
                    WoundPhase::Healed,
                ];

                prop_assert_eq!(
                    wound.phase,
                    phases[expected_phase_idx],
                    "Agent {} wound phase mismatch",
                    i
                );
            }
        }

        /// PROPERTY: Multiple wounds per agent are tracked separately.
        #[test]
        fn prop_multiple_wounds_per_agent(wound_count in 1usize..5) {
            let mut engine = make_engine();

            let agent_did = "did:agent:multi-wound".to_string();

            // Create multiple wounds for same agent
            for i in 0..wound_count {
                engine.create_wound(
                    agent_did.clone(),
                    WoundSeverity::Minor,
                    format!("cause-{}", i),
                ).unwrap();
            }

            // Verify all wounds are tracked
            let agent_wounds = engine.get_wounds_for_agent(&agent_did);
            prop_assert_eq!(
                agent_wounds.len(),
                wound_count,
                "Expected {} wounds, got {}",
                wound_count,
                agent_wounds.len()
            );
        }
    }
}
