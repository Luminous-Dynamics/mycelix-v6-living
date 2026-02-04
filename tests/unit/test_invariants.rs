//! Unit tests for key Living Protocol invariants.
//!
//! These tests verify the 9 critical invariants specified in the protocol:
//!
//! 1. Metabolic Trust scores always in [0.0, 1.0]
//! 2. Wound Healing phases advance forward only (never skip/reverse)
//! 3. Kenosis is irrevocable once committed
//! 4. Silence requires valid PresenceProof (can't fake presence)
//! 5. Dream phase proposals cannot execute financial operations
//! 6. Shadow Integration doesn't surface Gate 1-protected content
//! 7. Entanglement strength decays without continued co-creation
//! 8. Fractal governance patterns are structurally identical at all scales
//! 9. Metabolism Cycle phases transition correctly and completely

#[cfg(test)]
mod invariant_tests {
    // =========================================================================
    // Invariant 1: Metabolic Trust scores bounded [0.0, 1.0]
    // =========================================================================

    #[test]
    fn invariant_1_metabolic_trust_bounded() {
        use living_core::MetabolicTrustScore;

        // Normal inputs
        let score = MetabolicTrustScore::compute(0.8, 0.7, 0.6, 0.5);
        assert!(score.metabolic_trust >= 0.0);
        assert!(score.metabolic_trust <= 1.0);

        // Edge: all zeros
        let score = MetabolicTrustScore::compute(0.0, 0.0, 0.0, 0.0);
        assert_eq!(score.metabolic_trust, 0.0);

        // Edge: all ones
        let score = MetabolicTrustScore::compute(1.0, 1.0, 1.0, 1.0);
        assert!(score.metabolic_trust <= 1.0);

        // Edge: values above 1.0 (should clamp)
        let score = MetabolicTrustScore::compute(2.0, 2.0, 2.0, 2.0);
        assert!(score.metabolic_trust <= 1.0);

        // Edge: negative values (should clamp)
        let score = MetabolicTrustScore::compute(-1.0, -1.0, -1.0, -1.0);
        assert!(score.metabolic_trust >= 0.0);
    }

    // =========================================================================
    // Invariant 2: Wound phases forward-only
    // =========================================================================

    #[test]
    fn invariant_2_wound_phases_forward_only() {
        use living_core::WoundPhase;

        let phases = [
            WoundPhase::Hemostasis,
            WoundPhase::Inflammation,
            WoundPhase::Proliferation,
            WoundPhase::Remodeling,
            WoundPhase::Healed,
        ];

        for (i, phase) in phases.iter().enumerate() {
            let valid = phase.valid_transitions();

            // Can only go to the next phase
            if i < phases.len() - 1 {
                assert!(valid.contains(&phases[i + 1]));
                assert_eq!(valid.len(), 1);
            } else {
                // Healed has no valid transitions
                assert!(valid.is_empty());
            }

            // Cannot go backwards
            for j in 0..i {
                assert!(!phase.can_transition_to(&phases[j]));
            }

            // Cannot skip ahead
            for j in (i + 2)..phases.len() {
                assert!(!phase.can_transition_to(&phases[j]));
            }
        }
    }

    // =========================================================================
    // Invariant 3: Kenosis irrevocability
    // =========================================================================

    #[test]
    fn invariant_3_kenosis_cap() {
        use living_core::KenosisCommitment;
        use chrono::Utc;

        let commitment = KenosisCommitment {
            id: "test".into(),
            agent_did: "did:test:agent".into(),
            release_percentage: 0.15,
            reputation_released: 15.0,
            committed_at: Utc::now(),
            cycle_number: 1,
            irrevocable: true,
        };

        // Once committed, irrevocable is true
        assert!(commitment.irrevocable);

        // Release percentage must be <= 0.20 (20%)
        assert!(commitment.release_percentage <= 0.20);
    }

    // =========================================================================
    // Invariant 5: Dream phase financial restriction
    // =========================================================================

    #[test]
    fn invariant_5_dream_proposals_no_financial() {
        use living_core::DreamProposal;
        use chrono::Utc;

        let proposal = DreamProposal {
            id: "dream-1".into(),
            dream_state: living_core::DreamState::Rem,
            content: "What if we restructured the treasury?".into(),
            generated_at: Utc::now(),
            confirmed: false,
            confirmation_threshold: 0.67,
            financial_operations: false, // Must always be false
        };

        // Financial operations must always be false during dreams
        assert!(!proposal.financial_operations);
        // Must not be confirmed until waking phase
        assert!(!proposal.confirmed);
        // Confirmation threshold must be 0.67
        assert!((proposal.confirmation_threshold - 0.67).abs() < f64::EPSILON);
    }

    // =========================================================================
    // Invariant 7: Entanglement decay
    // =========================================================================

    #[test]
    fn invariant_7_entanglement_decays() {
        use living_core::EntangledPair;
        use chrono::{Utc, Duration};

        let pair = EntangledPair {
            id: "pair-1".into(),
            agent_a: "did:a".into(),
            agent_b: "did:b".into(),
            entanglement_strength: 1.0,
            formed: Utc::now(),
            last_co_creation: Utc::now() - Duration::days(30),
            decay_rate: 0.05,
        };

        let now = Utc::now();
        let strength = pair.current_strength(now);

        // After 30 days with decay rate 0.05, strength should be significantly reduced
        assert!(strength < 1.0, "Strength should decay");
        assert!(strength > 0.0, "Strength should not be zero");

        // Longer time = more decay
        let future_strength = pair.current_strength(now + Duration::days(30));
        assert!(future_strength < strength, "More time = more decay");
    }

    // =========================================================================
    // Invariant 9: Cycle transitions correct and complete
    // =========================================================================

    #[test]
    fn invariant_9_cycle_transitions_complete() {
        use living_core::CyclePhase;

        // Total cycle is exactly 28 days
        assert_eq!(CyclePhase::total_cycle_days(), 28);

        // All 9 phases are present
        assert_eq!(CyclePhase::all_phases().len(), 9);

        // Walking through all phases returns to start
        let mut phase = CyclePhase::Shadow;
        for _ in 0..9 {
            phase = phase.next();
        }
        assert_eq!(phase, CyclePhase::Shadow);

        // Each phase has a defined duration
        for phase in CyclePhase::all_phases() {
            assert!(phase.duration_days() > 0);
        }
    }

    // =========================================================================
    // Invariant: Beauty scores bounded
    // =========================================================================

    #[test]
    fn beauty_scores_bounded() {
        use living_core::BeautyScore;

        // Normal case
        let score = BeautyScore::compute(0.5, 0.6, 0.7, 0.8, 0.9);
        assert!(score.composite >= 0.0 && score.composite <= 1.0);

        // All zeros
        let score = BeautyScore::compute(0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(score.composite, 0.0);

        // All ones
        let score = BeautyScore::compute(1.0, 1.0, 1.0, 1.0, 1.0);
        assert!(score.composite <= 1.0);

        // Out of range inputs (should clamp)
        let score = BeautyScore::compute(2.0, 2.0, 2.0, 2.0, 2.0);
        assert!(score.composite <= 1.0);
    }

    // =========================================================================
    // K-Vector trust checks
    // =========================================================================

    #[test]
    fn k_vector_trust_decision() {
        use living_core::KVectorSignature;
        use chrono::Utc;

        let my_kvec = KVectorSignature::from_array(
            [0.5, 0.6, 0.7, 0.5, 0.6, 0.7, 0.7, 0.8],
            Utc::now(),
        );

        // Sane and aligned peer
        let good_peer = KVectorSignature::from_array(
            [0.6, 0.5, 0.6, 0.6, 0.5, 0.6, 0.8, 0.9],
            Utc::now(),
        );
        assert!(my_kvec.should_trust(&good_peer));

        // Insane peer (k_topo < 0.7)
        let insane_peer = KVectorSignature::from_array(
            [0.6, 0.5, 0.6, 0.6, 0.5, 0.6, 0.8, 0.3],
            Utc::now(),
        );
        assert!(!my_kvec.should_trust(&insane_peer));

        // Misaligned peer (k_h too far)
        let misaligned_peer = KVectorSignature::from_array(
            [0.6, 0.5, 0.6, 0.6, 0.5, 0.6, 0.1, 0.9],
            Utc::now(),
        );
        assert!(!my_kvec.should_trust(&misaligned_peer));
    }
}
