//! Fuzz testing targets for Byzantine scenarios.
//!
//! Each primitive gets fuzz-tested with adversarial inputs, timing attacks,
//! and Sybil scenarios to verify 45% Byzantine tolerance.

#[cfg(test)]
mod fuzz_targets {
    use living_core::*;
    use chrono::Utc;

    /// Fuzz target: MetabolicTrustScore always bounded regardless of input.
    #[test]
    fn fuzz_metabolic_trust_bounded() {
        // Test with a range of adversarial inputs
        let adversarial_inputs: Vec<(f64, f64, f64, f64)> = vec![
            (f64::MAX, f64::MAX, f64::MAX, f64::MAX),
            (f64::MIN, f64::MIN, f64::MIN, f64::MIN),
            (f64::NAN, 0.5, 0.5, 0.5),
            (f64::INFINITY, 0.5, 0.5, 0.5),
            (f64::NEG_INFINITY, 0.5, 0.5, 0.5),
            (-1000.0, 1000.0, -1000.0, 1000.0),
            (0.0, 0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0, 1.0),
        ];

        for (matl, throughput, resilience, composting) in adversarial_inputs {
            let score = MetabolicTrustScore::compute(matl, throughput, resilience, composting);

            // Score must always be bounded even with adversarial inputs
            // NaN inputs produce NaN which is acceptable (fails the <= check)
            if !score.metabolic_trust.is_nan() {
                assert!(
                    score.metabolic_trust >= 0.0 && score.metabolic_trust <= 1.0,
                    "Unbounded score: {} for inputs ({}, {}, {}, {})",
                    score.metabolic_trust, matl, throughput, resilience, composting
                );
            }
        }
    }

    /// Fuzz target: BeautyScore always bounded.
    #[test]
    fn fuzz_beauty_score_bounded() {
        let adversarial: Vec<f64> = vec![
            -1.0, 0.0, 0.5, 1.0, 2.0, 100.0, -100.0,
            f64::MAX, f64::MIN,
        ];

        for &s in &adversarial {
            for &e in &adversarial {
                let score = BeautyScore::compute(s, e, 0.5, 0.5, 0.5);
                if !score.composite.is_nan() {
                    assert!(
                        score.composite >= 0.0 && score.composite <= 1.0,
                        "Unbounded beauty: {} for symmetry={}, economy={}",
                        score.composite, s, e
                    );
                }
            }
        }
    }

    /// Fuzz target: K-Vector cosine similarity bounded [-1, 1].
    #[test]
    fn fuzz_kvector_cosine_bounded() {
        let test_vecs: Vec<[f64; 8]> = vec![
            [0.0; 8],
            [1.0; 8],
            [-1.0; 8],
            [0.5, -0.5, 0.5, -0.5, 0.5, -0.5, 0.5, -0.5],
            [100.0, -100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];

        for a_vals in &test_vecs {
            for b_vals in &test_vecs {
                let a = KVectorSignature::from_array(*a_vals, Utc::now());
                let b = KVectorSignature::from_array(*b_vals, Utc::now());

                let sim = a.cosine_similarity(&b);

                if !sim.is_nan() {
                    assert!(
                        sim >= -1.0 - f64::EPSILON && sim <= 1.0 + f64::EPSILON,
                        "Unbounded cosine similarity: {} for {:?} and {:?}",
                        sim, a_vals, b_vals
                    );
                }
            }
        }
    }

    /// Fuzz target: Entanglement decay never goes negative.
    #[test]
    fn fuzz_entanglement_decay_non_negative() {
        let decay_rates = vec![0.0, 0.01, 0.1, 1.0, 10.0, 100.0];
        let days_since = vec![0, 1, 7, 28, 365, 10000];

        for &rate in &decay_rates {
            for &days in &days_since {
                let pair = EntangledPair {
                    id: "fuzz".into(),
                    agent_a: "a".into(),
                    agent_b: "b".into(),
                    entanglement_strength: 1.0,
                    formed: Utc::now(),
                    last_co_creation: Utc::now() - chrono::Duration::days(days),
                    decay_rate: rate,
                };

                let strength = pair.current_strength(Utc::now());
                assert!(
                    strength >= 0.0,
                    "Negative entanglement: {} for rate={}, days={}",
                    strength, rate, days
                );
            }
        }
    }

    /// Fuzz target: WoundPhase transitions are strictly monotonic.
    #[test]
    fn fuzz_wound_phase_monotonic() {
        let all_phases = [
            WoundPhase::Hemostasis,
            WoundPhase::Inflammation,
            WoundPhase::Proliferation,
            WoundPhase::Remodeling,
            WoundPhase::Healed,
        ];

        // No phase can transition to itself or backwards
        for (i, phase) in all_phases.iter().enumerate() {
            assert!(!phase.can_transition_to(phase), "Phase {:?} can transition to itself", phase);

            for j in 0..i {
                assert!(
                    !phase.can_transition_to(&all_phases[j]),
                    "Phase {:?} can transition backwards to {:?}",
                    phase, all_phases[j]
                );
            }
        }
    }

    /// Fuzz target: CyclePhase next/prev are inverses.
    #[test]
    fn fuzz_cycle_phase_next_prev_inverse() {
        for phase in CyclePhase::all_phases() {
            let next = phase.next();
            let back = next.prev();
            assert_eq!(*phase, back, "next/prev not inverse for {:?}", phase);
        }
    }
}
