//! Integration test: Full Metabolism Cycle
//!
//! Tests that the 28-day metabolism cycle completes correctly,
//! all phase transitions fire in order, and inter-module events propagate.

#[cfg(test)]
mod metabolism_cycle_tests {
    use living_core::*;
    use cycle_engine::*;

    fn setup_engine() -> MetabolismCycleEngine {
        CycleEngineBuilder::new()
            .with_simulated_time(86400.0) // 1 second = 1 day
            .build()
    }

    #[test]
    fn test_full_28_day_cycle() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        let expected_phases = [
            CyclePhase::Composting,
            CyclePhase::Liminal,
            CyclePhase::NegativeCapability,
            CyclePhase::Eros,
            CyclePhase::CoCreation,
            CyclePhase::Beauty,
            CyclePhase::EmergentPersonhood,
            CyclePhase::Kenosis,
            CyclePhase::Shadow, // wraps back
        ];

        for expected in &expected_phases {
            let events = engine.force_transition().unwrap();
            assert_eq!(engine.current_phase(), *expected);

            // Every transition should produce at least one event
            assert!(!events.is_empty(), "Phase {:?} produced no events", expected);
        }

        // We should now be in cycle 2
        assert_eq!(engine.cycle_number(), 2);

        // Transition history should have 9 entries
        assert_eq!(engine.transition_history().len(), 9);
    }

    #[test]
    fn test_phase_duration_sum_is_28() {
        assert_eq!(CyclePhase::total_cycle_days(), 28);
    }

    #[test]
    fn test_voting_blocked_in_negative_capability() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        // Advance to NegativeCapability (Shadow -> Composting -> Liminal -> NegCap)
        engine.force_transition().unwrap(); // Composting
        engine.force_transition().unwrap(); // Liminal
        engine.force_transition().unwrap(); // NegativeCapability

        assert_eq!(engine.current_phase(), CyclePhase::NegativeCapability);
        assert!(!engine.is_operation_permitted("vote"));
        assert!(engine.is_operation_permitted("read"));
    }

    #[test]
    fn test_gate2_suspended_in_shadow() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        assert_eq!(engine.current_phase(), CyclePhase::Shadow);
        assert!(!engine.is_operation_permitted("gate2_warning"));
    }

    #[test]
    fn test_kenosis_only_in_kenosis_phase() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        // In Shadow phase, kenosis should not be the phase
        assert_ne!(engine.current_phase(), CyclePhase::Kenosis);

        // Advance through all phases to Kenosis
        for _ in 0..8 {
            engine.force_transition().unwrap();
        }

        assert_eq!(engine.current_phase(), CyclePhase::Kenosis);
        assert!(engine.is_operation_permitted("kenosis"));
    }

    #[test]
    fn test_multiple_cycles() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        // Run 3 full cycles
        for cycle in 1..=3 {
            for _ in 0..9 {
                engine.force_transition().unwrap();
            }
            assert_eq!(engine.cycle_number(), cycle + 1);
            assert_eq!(engine.current_phase(), CyclePhase::Shadow);
        }
    }

    #[test]
    fn test_transition_metrics_recorded() {
        let mut engine = setup_engine();
        engine.start().unwrap();

        engine.force_transition().unwrap();

        let history = engine.transition_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from, CyclePhase::Shadow);
        assert_eq!(history[0].to, CyclePhase::Composting);
        assert_eq!(history[0].cycle_number, 1);
    }
}
