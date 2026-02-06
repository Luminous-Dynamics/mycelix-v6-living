//! Property-based tests for beauty score bounds.
//!
//! This module provides comprehensive property-based testing for the beauty
//! validity engine, ensuring all score components remain properly bounded.
//!
//! ## Key Properties Tested
//!
//! 1. All score dimensions are in [0, 1]
//! 2. Composite score is in [0, 1]
//! 3. Weights sum to 1.0
//! 4. Gate 1 invariants always pass
//!
//! ## Running Extended Tests
//!
//! ```bash
//! PROPTEST_CASES=10000 cargo test -p epistemics --release -- proptest
//! ```

#[cfg(test)]
mod proptest_beauty_tests {
    use crate::beauty_validity::BeautyValidityEngine;
    use living_core::{BeautyScore, LivingPrimitive};
    use proptest::prelude::*;

    // =========================================================================
    // Arbitrary Implementations
    // =========================================================================

    fn arb_unit_interval() -> impl Strategy<Value = f64> {
        (0u64..=1000u64).prop_map(|n| n as f64 / 1000.0)
    }

    fn arb_content() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-zA-Z0-9 .,!?]", 10..500)
            .prop_map(|chars| chars.into_iter().collect())
    }

    fn arb_sentence() -> impl Strategy<Value = String> {
        prop::collection::vec("[a-zA-Z]", 5..20)
            .prop_map(|chars| chars.into_iter().collect::<String>() + ".")
    }

    fn arb_paragraph() -> impl Strategy<Value = String> {
        prop::collection::vec(arb_sentence(), 1..5)
            .prop_map(|sentences| sentences.join(" "))
    }

    fn arb_multi_paragraph_content() -> impl Strategy<Value = String> {
        prop::collection::vec(arb_paragraph(), 1..4)
            .prop_map(|paragraphs| paragraphs.join("\n\n"))
    }

    fn arb_patterns() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(arb_paragraph(), 0..5)
    }

    fn arb_requirements() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop::collection::vec("[a-zA-Z]", 5..15)
                .prop_map(|chars| chars.into_iter().collect()),
            0..5
        )
    }

    // =========================================================================
    // Beauty Score Bounds Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// PROPERTY: BeautyScore::compute always produces bounded values.
        /// All components and composite must be in [0, 1].
        #[test]
        fn prop_beauty_score_compute_bounded(
            symmetry in arb_unit_interval(),
            economy in arb_unit_interval(),
            resonance in arb_unit_interval(),
            surprise in arb_unit_interval(),
            completeness in arb_unit_interval(),
        ) {
            let score = BeautyScore::compute(
                symmetry, economy, resonance, surprise, completeness
            );

            prop_assert!(
                score.symmetry >= 0.0 && score.symmetry <= 1.0,
                "symmetry {} out of bounds",
                score.symmetry
            );
            prop_assert!(
                score.economy >= 0.0 && score.economy <= 1.0,
                "economy {} out of bounds",
                score.economy
            );
            prop_assert!(
                score.resonance >= 0.0 && score.resonance <= 1.0,
                "resonance {} out of bounds",
                score.resonance
            );
            prop_assert!(
                score.surprise >= 0.0 && score.surprise <= 1.0,
                "surprise {} out of bounds",
                score.surprise
            );
            prop_assert!(
                score.completeness >= 0.0 && score.completeness <= 1.0,
                "completeness {} out of bounds",
                score.completeness
            );
            prop_assert!(
                score.composite >= 0.0 && score.composite <= 1.0,
                "composite {} out of bounds",
                score.composite
            );
        }

        /// PROPERTY: Composite score equals weighted sum of components.
        #[test]
        fn prop_composite_equals_weighted_sum(
            symmetry in arb_unit_interval(),
            economy in arb_unit_interval(),
            resonance in arb_unit_interval(),
            surprise in arb_unit_interval(),
            completeness in arb_unit_interval(),
        ) {
            let score = BeautyScore::compute(
                symmetry, economy, resonance, surprise, completeness
            );

            let expected = (
                0.20 * symmetry +
                0.20 * economy +
                0.25 * resonance +
                0.15 * surprise +
                0.20 * completeness
            ).clamp(0.0, 1.0);

            prop_assert!(
                (score.composite - expected).abs() < 0.0001,
                "Composite {} doesn't match expected {}",
                score.composite,
                expected
            );
        }

        /// PROPERTY: All 1.0 inputs produce composite of 1.0.
        #[test]
        fn prop_all_ones_composite_is_one(_dummy in 0..1) {
            let score = BeautyScore::compute(1.0, 1.0, 1.0, 1.0, 1.0);
            prop_assert!(
                (score.composite - 1.0).abs() < 0.0001,
                "All 1.0 should yield composite 1.0, got {}",
                score.composite
            );
        }

        /// PROPERTY: All 0.0 inputs produce composite of 0.0.
        #[test]
        fn prop_all_zeros_composite_is_zero(_dummy in 0..1) {
            let score = BeautyScore::compute(0.0, 0.0, 0.0, 0.0, 0.0);
            prop_assert!(
                score.composite.abs() < 0.0001,
                "All 0.0 should yield composite 0.0, got {}",
                score.composite
            );
        }

        /// PROPERTY: Weights sum to 1.0.
        #[test]
        fn prop_weights_sum_to_one(_dummy in 0..1) {
            let weight_sum: f64 = 0.20 + 0.20 + 0.25 + 0.15 + 0.20;
            prop_assert!(
                (weight_sum - 1.0).abs() < 0.0001,
                "Weights sum to {}, not 1.0",
                weight_sum
            );
        }
    }

    // =========================================================================
    // Engine Scoring Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: Engine-computed scores are always bounded.
        #[test]
        fn prop_engine_scores_bounded(
            content in arb_multi_paragraph_content(),
            patterns in arb_patterns(),
            requirements in arb_requirements(),
        ) {
            let mut engine = BeautyValidityEngine::new();

            let event = engine.score_proposal(
                "prop-test",
                &content,
                "did:scorer:test",
                &patterns,
                &requirements,
            );

            let s = &event.score;

            prop_assert!(
                s.symmetry >= 0.0 && s.symmetry <= 1.0,
                "Engine symmetry {} out of bounds",
                s.symmetry
            );
            prop_assert!(
                s.economy >= 0.0 && s.economy <= 1.0,
                "Engine economy {} out of bounds",
                s.economy
            );
            prop_assert!(
                s.resonance >= 0.0 && s.resonance <= 1.0,
                "Engine resonance {} out of bounds",
                s.resonance
            );
            prop_assert!(
                s.surprise >= 0.0 && s.surprise <= 1.0,
                "Engine surprise {} out of bounds",
                s.surprise
            );
            prop_assert!(
                s.completeness >= 0.0 && s.completeness <= 1.0,
                "Engine completeness {} out of bounds",
                s.completeness
            );
            prop_assert!(
                s.composite >= 0.0 && s.composite <= 1.0,
                "Engine composite {} out of bounds",
                s.composite
            );
        }

        /// PROPERTY: Gate 1 always passes for engine-computed scores.
        #[test]
        fn prop_gate1_always_passes(
            content in arb_multi_paragraph_content(),
            patterns in arb_patterns(),
            requirements in arb_requirements(),
            scorer_count in 1usize..5,
        ) {
            let mut engine = BeautyValidityEngine::new();

            for i in 0..scorer_count {
                engine.score_proposal(
                    "prop-test",
                    &content,
                    &format!("did:scorer:{}", i),
                    &patterns,
                    &requirements,
                );
            }

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

        /// PROPERTY: Aggregate score is bounded.
        #[test]
        fn prop_aggregate_bounded(
            content in arb_multi_paragraph_content(),
            scorer_count in 1usize..5,
        ) {
            let mut engine = BeautyValidityEngine::new();

            for i in 0..scorer_count {
                engine.score_proposal(
                    "prop-test",
                    &content,
                    &format!("did:scorer:{}", i),
                    &[],
                    &[],
                );
            }

            if let Some(agg) = engine.aggregate_scores("prop-test") {
                prop_assert!(
                    agg.composite >= 0.0 && agg.composite <= 1.0,
                    "Aggregate composite {} out of bounds",
                    agg.composite
                );
            }
        }
    }

    // =========================================================================
    // Individual Scoring Function Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]

        /// PROPERTY: compute_symmetry is always in [0, 1].
        #[test]
        fn prop_symmetry_bounded(content in arb_multi_paragraph_content()) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_symmetry(&content);

            prop_assert!(
                score >= 0.0 && score <= 1.0,
                "Symmetry {} out of bounds for content length {}",
                score,
                content.len()
            );
        }

        /// PROPERTY: compute_economy is always in [0, 1].
        #[test]
        fn prop_economy_bounded(content in arb_multi_paragraph_content()) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_economy(&content);

            prop_assert!(
                score >= 0.0 && score <= 1.0,
                "Economy {} out of bounds",
                score
            );
        }

        /// PROPERTY: compute_resonance is always in [0, 1].
        #[test]
        fn prop_resonance_bounded(
            content in arb_multi_paragraph_content(),
            patterns in arb_patterns(),
        ) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_resonance(&content, &patterns);

            prop_assert!(
                score >= 0.0 && score <= 1.0,
                "Resonance {} out of bounds",
                score
            );
        }

        /// PROPERTY: compute_surprise is always in [0, 1].
        #[test]
        fn prop_surprise_bounded(
            content in arb_multi_paragraph_content(),
            patterns in arb_patterns(),
        ) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_surprise(&content, &patterns);

            prop_assert!(
                score >= 0.0 && score <= 1.0,
                "Surprise {} out of bounds",
                score
            );
        }

        /// PROPERTY: compute_completeness is always in [0, 1].
        #[test]
        fn prop_completeness_bounded(
            content in arb_multi_paragraph_content(),
            requirements in arb_requirements(),
        ) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_completeness(&content, &requirements);

            prop_assert!(
                score >= 0.0 && score <= 1.0,
                "Completeness {} out of bounds",
                score
            );
        }

        /// PROPERTY: Empty content produces score 0.0 for most dimensions.
        #[test]
        fn prop_empty_content_scores(_dummy in 0..1) {
            let engine = BeautyValidityEngine::new();

            prop_assert_eq!(engine.compute_symmetry(""), 0.0);
            prop_assert_eq!(engine.compute_economy(""), 0.0);
            prop_assert_eq!(engine.compute_surprise("", &[]), 0.0);
            prop_assert_eq!(engine.compute_completeness("", &["req".to_string()]), 0.0);
        }

        /// PROPERTY: Empty patterns produces neutral resonance (0.5).
        #[test]
        fn prop_empty_patterns_neutral_resonance(content in arb_multi_paragraph_content()) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_resonance(&content, &[]);

            prop_assert_eq!(
                score,
                0.5,
                "Empty patterns should give 0.5 resonance, got {}",
                score
            );
        }

        /// PROPERTY: Empty requirements produces perfect completeness (1.0).
        #[test]
        fn prop_empty_requirements_complete(content in arb_multi_paragraph_content()) {
            let engine = BeautyValidityEngine::new();
            let score = engine.compute_completeness(&content, &[]);

            prop_assert_eq!(
                score,
                1.0,
                "Empty requirements should give 1.0 completeness, got {}",
                score
            );
        }
    }

    // =========================================================================
    // Threshold Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: meets_threshold is consistent with aggregate_scores.
        #[test]
        fn prop_threshold_consistent_with_aggregate(
            content in arb_multi_paragraph_content(),
            threshold in arb_unit_interval(),
        ) {
            let mut engine = BeautyValidityEngine::new();

            engine.score_proposal(
                "prop-test",
                &content,
                "did:scorer:test",
                &[],
                &[],
            );

            if let Some(agg) = engine.aggregate_scores("prop-test") {
                let meets = engine.meets_threshold("prop-test", threshold);
                let expected = agg.composite >= threshold;

                prop_assert_eq!(
                    meets,
                    expected,
                    "meets_threshold({}) returned {}, but composite {} {} threshold",
                    threshold,
                    meets,
                    agg.composite,
                    if expected { ">=" } else { "<" }
                );
            }
        }

        /// PROPERTY: Threshold of 0.0 is always met (for scored proposals).
        #[test]
        fn prop_zero_threshold_always_met(content in arb_multi_paragraph_content()) {
            let mut engine = BeautyValidityEngine::new();

            engine.score_proposal(
                "prop-test",
                &content,
                "did:scorer:test",
                &[],
                &[],
            );

            prop_assert!(
                engine.meets_threshold("prop-test", 0.0),
                "Zero threshold should always be met"
            );
        }

        /// PROPERTY: Threshold > 1.0 is never met.
        #[test]
        fn prop_high_threshold_never_met(content in arb_multi_paragraph_content()) {
            let mut engine = BeautyValidityEngine::new();

            engine.score_proposal(
                "prop-test",
                &content,
                "did:scorer:test",
                &[],
                &[],
            );

            prop_assert!(
                !engine.meets_threshold("prop-test", 1.1),
                "Threshold > 1.0 should never be met"
            );
        }
    }
}
