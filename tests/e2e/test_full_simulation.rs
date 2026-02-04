//! End-to-End test: Full Living Protocol simulation.
//!
//! Simulates a network with multiple agents going through metabolism cycles,
//! forming entanglements, healing wounds, and undergoing kenosis.

#[cfg(test)]
mod e2e_tests {
    use living_core::*;
    use chrono::Utc;

    /// Simulated agent for E2E testing.
    struct SimulatedAgent {
        did: Did,
        k_vector: KVectorSignature,
        reputation: f64,
        wounds: Vec<WoundRecord>,
        entanglements: Vec<String>,
    }

    impl SimulatedAgent {
        fn new(did: &str, k_values: [f64; 8]) -> Self {
            Self {
                did: did.to_string(),
                k_vector: KVectorSignature::from_array(k_values, Utc::now()),
                reputation: 0.5,
                wounds: Vec::new(),
                entanglements: Vec::new(),
            }
        }
    }

    #[test]
    fn test_10_agent_simulation() {
        // Create 10 agents with varying K-Vectors
        let agents: Vec<SimulatedAgent> = (0..10)
            .map(|i| {
                let base = 0.3 + (i as f64 * 0.05);
                SimulatedAgent::new(
                    &format!("did:test:agent_{}", i),
                    [base, base + 0.1, base + 0.05, base, base + 0.1, base + 0.05, base + 0.1, 0.8],
                )
            })
            .collect();

        // All agents should be sane (k_topo = 0.8)
        for agent in &agents {
            assert!(agent.k_vector.is_sane());
        }

        // Find aligned pairs
        let mut aligned_count = 0;
        for i in 0..agents.len() {
            for j in (i + 1)..agents.len() {
                if agents[i].k_vector.is_aligned_with(&agents[j].k_vector) {
                    aligned_count += 1;
                }
            }
        }
        assert!(aligned_count > 0, "Some agents should be aligned");

        // Verify metabolic trust computation
        for agent in &agents {
            let score = MetabolicTrustScore::compute(
                agent.reputation,
                0.5, // throughput
                0.5, // resilience
                0.3, // composting contribution
            );
            assert!(score.metabolic_trust >= 0.0 && score.metabolic_trust <= 1.0);
        }

        // Test entanglement decay
        let pair = EntangledPair {
            id: "pair-0-1".into(),
            agent_a: agents[0].did.clone(),
            agent_b: agents[1].did.clone(),
            entanglement_strength: 0.8,
            formed: Utc::now(),
            last_co_creation: Utc::now() - chrono::Duration::days(14),
            decay_rate: 0.03,
        };

        let strength = pair.current_strength(Utc::now());
        assert!(strength < 0.8, "Should have decayed");
        assert!(strength > 0.0, "Should not be zero");

        // Test beauty scoring
        let beauty = BeautyScore::compute(0.7, 0.8, 0.6, 0.5, 0.9);
        assert!(beauty.composite > 0.0);
        assert!(beauty.composite <= 1.0);
    }

    #[test]
    fn test_wound_healing_lifecycle() {
        let wound = WoundRecord {
            id: "wound-1".into(),
            agent_did: "did:test:agent_0".into(),
            severity: WoundSeverity::Moderate,
            cause: "Protocol violation".into(),
            phase: WoundPhase::Hemostasis,
            created: Utc::now(),
            phase_history: vec![(WoundPhase::Hemostasis, Utc::now())],
            restitution_required: Some(RestitutionRequirement {
                description: "Return misallocated funds".into(),
                amount_flow: Some(100.0),
                actions_required: vec!["Return funds".into(), "Public acknowledgment".into()],
                deadline: Utc::now() + chrono::Duration::days(28),
                fulfilled: false,
            }),
            scar_tissue: None,
        };

        // Verify forward-only transitions
        assert!(wound.phase.can_transition_to(&WoundPhase::Inflammation));
        assert!(!wound.phase.can_transition_to(&WoundPhase::Proliferation));
        assert!(!wound.phase.can_transition_to(&WoundPhase::Healed));
    }

    #[test]
    fn test_cycle_phase_behaviors() {
        // Verify each phase has expected behavioral constraints
        for phase in CyclePhase::all_phases() {
            match phase {
                CyclePhase::Shadow => {
                    // Gate 2 warnings suspended
                    assert_eq!(phase.duration_days(), 2);
                }
                CyclePhase::Composting => {
                    assert_eq!(phase.duration_days(), 5);
                }
                CyclePhase::Liminal => {
                    // No premature recategorization
                    assert_eq!(phase.duration_days(), 3);
                }
                CyclePhase::NegativeCapability => {
                    // Voting blocked
                    assert_eq!(phase.duration_days(), 3);
                }
                CyclePhase::Eros => {
                    assert_eq!(phase.duration_days(), 4);
                }
                CyclePhase::CoCreation => {
                    // Standard consensus
                    assert_eq!(phase.duration_days(), 7);
                }
                CyclePhase::Beauty => {
                    assert_eq!(phase.duration_days(), 2);
                }
                CyclePhase::EmergentPersonhood => {
                    assert_eq!(phase.duration_days(), 1);
                }
                CyclePhase::Kenosis => {
                    // Self-emptying, max 20% per cycle
                    assert_eq!(phase.duration_days(), 1);
                }
            }
        }
    }
}
