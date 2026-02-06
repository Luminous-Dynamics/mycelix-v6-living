//! Property-based tests for K-Vector normalization and temporal tracking.
//!
//! This module provides comprehensive property-based testing for K-Vector
//! operations, ensuring mathematical invariants are maintained.
//!
//! ## Key Properties Tested
//!
//! 1. K-Vector values remain bounded in [0, 1]
//! 2. Normalization preserves relative ordering
//! 3. Velocity and acceleration are finite
//! 4. Predictions remain bounded
//!
//! ## Running Extended Tests
//!
//! ```bash
//! PROPTEST_CASES=10000 cargo test -p consciousness --release -- proptest
//! ```

#[cfg(test)]
mod proptest_kvector_tests {
    use crate::temporal_k_vector::TemporalKVectorService;
    use living_core::{KVectorSignature, TemporalKVector};
    use living_core::traits::LivingPrimitive;
    use chrono::{Duration, Utc};
    use proptest::prelude::*;

    // =========================================================================
    // Arbitrary Implementations
    // =========================================================================

    fn arb_unit_interval() -> impl Strategy<Value = f64> {
        (0u64..=1000u64).prop_map(|n| n as f64 / 1000.0)
    }

    fn arb_kvector_values() -> impl Strategy<Value = [f64; 8]> {
        prop::array::uniform8(arb_unit_interval())
    }

    fn arb_positive_days() -> impl Strategy<Value = f64> {
        (1u64..=365u64).prop_map(|n| n as f64)
    }

    fn arb_small_positive() -> impl Strategy<Value = f64> {
        (1u64..=1000u64).prop_map(|n| n as f64 / 100.0)
    }

    fn make_kvec(values: [f64; 8], days_ago: i64) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now() - Duration::days(days_ago))
    }

    fn make_kvec_now(values: [f64; 8]) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now())
    }

    // =========================================================================
    // K-Vector Bounds Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// PROPERTY: K-Vector values are always in [0, 1].
        #[test]
        fn prop_kvector_values_bounded(values in arb_kvector_values()) {
            let kvec = make_kvec_now(values);
            let arr = kvec.as_array();

            for (i, val) in arr.iter().enumerate() {
                prop_assert!(
                    *val >= 0.0 && *val <= 1.0,
                    "K-Vector dimension {} value {} out of bounds",
                    i,
                    val
                );
            }
        }

        /// PROPERTY: K-Vector magnitude is bounded.
        /// Max magnitude = sqrt(8) when all dimensions are 1.0.
        #[test]
        fn prop_kvector_magnitude_bounded(values in arb_kvector_values()) {
            let kvec = make_kvec_now(values);
            let magnitude = kvec.magnitude();

            // Maximum magnitude when all 8 dimensions are 1.0: sqrt(8) ≈ 2.828
            let max_magnitude = (8.0_f64).sqrt();

            prop_assert!(
                magnitude >= 0.0 && magnitude <= max_magnitude + 0.001,
                "Magnitude {} exceeds maximum {}",
                magnitude,
                max_magnitude
            );
        }

        /// PROPERTY: Cosine similarity is in [-1, 1].
        #[test]
        fn prop_cosine_similarity_bounded(
            values_a in arb_kvector_values(),
            values_b in arb_kvector_values(),
        ) {
            let a = make_kvec_now(values_a);
            let b = make_kvec_now(values_b);

            let similarity = a.cosine_similarity(&b);

            prop_assert!(
                similarity >= -1.0 - 0.001 && similarity <= 1.0 + 0.001,
                "Cosine similarity {} out of bounds",
                similarity
            );
        }

        /// PROPERTY: Distance is non-negative.
        #[test]
        fn prop_distance_non_negative(
            values_a in arb_kvector_values(),
            values_b in arb_kvector_values(),
        ) {
            let a = make_kvec_now(values_a);
            let b = make_kvec_now(values_b);

            let distance = a.distance(&b);

            prop_assert!(
                distance >= 0.0,
                "Distance {} is negative",
                distance
            );
        }

        /// PROPERTY: Distance to self is zero.
        #[test]
        fn prop_distance_to_self_zero(values in arb_kvector_values()) {
            let kvec = make_kvec_now(values);
            let distance = kvec.distance(&kvec);

            prop_assert!(
                distance.abs() < 0.0001,
                "Distance to self {} is not zero",
                distance
            );
        }

        /// PROPERTY: Cosine similarity with self is 1.0 (for non-zero vectors).
        #[test]
        fn prop_cosine_self_is_one(values in arb_kvector_values()) {
            let kvec = make_kvec_now(values);

            // Skip zero vectors
            if kvec.magnitude() < 0.0001 {
                return Ok(());
            }

            let similarity = kvec.cosine_similarity(&kvec);

            prop_assert!(
                (similarity - 1.0).abs() < 0.0001,
                "Cosine similarity with self {} is not 1.0",
                similarity
            );
        }
    }

    // =========================================================================
    // Normalization Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// PROPERTY: Min-max normalization produces values in [0, 1].
        #[test]
        fn prop_normalization_bounded(values in prop::collection::vec(0.0f64..100.0f64, 8..=8)) {
            let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            if (max_val - min_val).abs() < 1e-10 {
                // All values equal, normalization undefined
                return Ok(());
            }

            let normalized: Vec<f64> = values.iter()
                .map(|v| (v - min_val) / (max_val - min_val))
                .collect();

            for (i, val) in normalized.iter().enumerate() {
                prop_assert!(
                    *val >= 0.0 - 0.0001 && *val <= 1.0 + 0.0001,
                    "Normalized value {} at index {} out of bounds",
                    val,
                    i
                );
            }
        }

        /// PROPERTY: Normalization preserves relative ordering.
        #[test]
        fn prop_normalization_preserves_order(values in prop::collection::vec(0.0f64..100.0f64, 8..=8)) {
            let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            if (max_val - min_val).abs() < 1e-10 {
                return Ok(());
            }

            let normalized: Vec<f64> = values.iter()
                .map(|v| (v - min_val) / (max_val - min_val))
                .collect();

            for i in 0..8 {
                for j in 0..8 {
                    if values[i] < values[j] - 1e-10 {
                        prop_assert!(
                            normalized[i] <= normalized[j] + 1e-10,
                            "Order not preserved: orig[{}]={} < orig[{}]={} but norm[{}]={} > norm[{}]={}",
                            i, values[i], j, values[j], i, normalized[i], j, normalized[j]
                        );
                    }
                }
            }
        }

        /// PROPERTY: L2 normalization produces unit vector.
        #[test]
        fn prop_l2_normalization_unit_vector(values in arb_kvector_values()) {
            let arr: [f64; 8] = values;
            let magnitude: f64 = arr.iter().map(|v| v * v).sum::<f64>().sqrt();

            if magnitude < 1e-10 {
                return Ok(());
            }

            let normalized: Vec<f64> = arr.iter().map(|v| v / magnitude).collect();
            let new_magnitude: f64 = normalized.iter().map(|v| v * v).sum::<f64>().sqrt();

            prop_assert!(
                (new_magnitude - 1.0).abs() < 0.0001,
                "L2 normalized magnitude {} is not 1.0",
                new_magnitude
            );
        }
    }

    // =========================================================================
    // Temporal K-Vector Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]

        /// PROPERTY: Velocity components are finite.
        #[test]
        fn prop_velocity_finite(
            initial_values in arb_kvector_values(),
            update_values in arb_kvector_values(),
        ) {
            let initial = make_kvec(initial_values, 1);
            let mut temporal = TemporalKVector::new(initial, 100);

            let update = make_kvec_now(update_values);
            temporal.update(update);

            for (i, vel) in temporal.velocity.iter().enumerate() {
                prop_assert!(
                    vel.is_finite(),
                    "Velocity dimension {} is not finite: {}",
                    i,
                    vel
                );
            }
        }

        /// PROPERTY: Acceleration components are finite.
        #[test]
        fn prop_acceleration_finite(
            initial_values in arb_kvector_values(),
            update1_values in arb_kvector_values(),
            update2_values in arb_kvector_values(),
        ) {
            let base_time = Utc::now() - Duration::days(2);

            let initial = KVectorSignature::from_array(initial_values, base_time);
            let mut temporal = TemporalKVector::new(initial, 100);

            let update1 = KVectorSignature::from_array(
                update1_values,
                base_time + Duration::days(1)
            );
            temporal.update(update1);

            let update2 = KVectorSignature::from_array(
                update2_values,
                base_time + Duration::days(2)
            );
            temporal.update(update2);

            for (i, acc) in temporal.acceleration.iter().enumerate() {
                prop_assert!(
                    acc.is_finite(),
                    "Acceleration dimension {} is not finite: {}",
                    i,
                    acc
                );
            }
        }

        /// PROPERTY: Rate of change is non-negative.
        #[test]
        fn prop_rate_of_change_non_negative(
            initial_values in arb_kvector_values(),
            update_values in arb_kvector_values(),
        ) {
            let initial = make_kvec(initial_values, 1);
            let mut temporal = TemporalKVector::new(initial, 100);

            let update = make_kvec_now(update_values);
            temporal.update(update);

            let roc = temporal.rate_of_change();
            prop_assert!(
                roc >= 0.0,
                "Rate of change {} is negative",
                roc
            );
        }

        /// PROPERTY: Predictions are bounded in [0, 1].
        #[test]
        fn prop_predictions_bounded(
            initial_values in arb_kvector_values(),
            update_values in arb_kvector_values(),
            days_forward in arb_positive_days(),
        ) {
            let initial = make_kvec(initial_values, 1);
            let mut temporal = TemporalKVector::new(initial, 100);

            let update = make_kvec_now(update_values);
            temporal.update(update);

            let predicted = temporal.predict(days_forward);

            for (i, val) in predicted.iter().enumerate() {
                prop_assert!(
                    *val >= 0.0 && *val <= 1.0,
                    "Predicted dimension {} value {} out of bounds for {} days forward",
                    i,
                    val,
                    days_forward
                );
            }
        }

        /// PROPERTY: Zero velocity means prediction equals current.
        #[test]
        fn prop_zero_velocity_constant_prediction(values in arb_kvector_values()) {
            let kvec = make_kvec_now(values);
            let temporal = TemporalKVector::new(kvec.clone(), 100);

            // No updates, so velocity = 0
            prop_assert_eq!(temporal.velocity, [0.0; 8]);

            let predicted = temporal.predict(10.0);
            let current = temporal.current.as_array();

            for i in 0..8 {
                prop_assert!(
                    (predicted[i] - current[i]).abs() < 0.0001,
                    "With zero velocity, prediction should equal current: predicted[{}]={}, current[{}]={}",
                    i, predicted[i], i, current[i]
                );
            }
        }
    }

    // =========================================================================
    // Service Properties
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// PROPERTY: Registered agents are retrievable.
        #[test]
        fn prop_registered_agents_retrievable(
            agent_count in 1usize..10,
            values in arb_kvector_values(),
        ) {
            let mut service = TemporalKVectorService::new();

            for i in 0..agent_count {
                let did = format!("did:agent:{}", i);
                let kvec = make_kvec_now(values);
                service.register_agent(&did, kvec);
            }

            prop_assert_eq!(service.agent_count(), agent_count);

            for i in 0..agent_count {
                let did = format!("did:agent:{}", i);
                prop_assert!(
                    service.is_registered(&did),
                    "Agent {} should be registered",
                    did
                );
            }
        }

        /// PROPERTY: Unregistered agents are not retrievable.
        #[test]
        fn prop_unregistered_not_retrievable(values in arb_kvector_values()) {
            let mut service = TemporalKVectorService::new();

            let did = "did:agent:test".to_string();
            let kvec = make_kvec_now(values);
            service.register_agent(&did, kvec);

            service.unregister_agent(&did);

            prop_assert!(!service.is_registered(&did));
            prop_assert!(service.get_velocity(&did).is_err());
        }

        /// PROPERTY: Gate 1 checks pass for valid service state.
        #[test]
        fn prop_gate1_passes(
            agent_count in 1usize..5,
            values in arb_kvector_values(),
        ) {
            let mut service = TemporalKVectorService::new();

            for i in 0..agent_count {
                let did = format!("did:agent:{}", i);
                let initial = make_kvec(values, 1);
                service.register_agent(&did, initial);

                // Add an update
                let update = make_kvec_now(values);
                let _ = service.update_observation(&did, update);
            }

            let checks = service.gate1_check();
            for check in &checks {
                prop_assert!(
                    check.passed,
                    "Gate 1 failed: {} - {:?}",
                    check.invariant,
                    check.details
                );
            }
        }

        /// PROPERTY: Most volatile dimensions returns valid indices.
        #[test]
        fn prop_volatile_dimensions_valid(
            agent_count in 1usize..5,
            initial_values in arb_kvector_values(),
            update_values in arb_kvector_values(),
        ) {
            let mut service = TemporalKVectorService::new();

            for i in 0..agent_count {
                let did = format!("did:agent:{}", i);
                let initial = make_kvec(initial_values, 1);
                service.register_agent(&did, initial);

                let update = make_kvec_now(update_values);
                let _ = service.update_observation(&did, update);
            }

            let volatile = service.most_volatile_dimensions_global(3);

            for (dim_idx, _volatility) in &volatile {
                prop_assert!(
                    *dim_idx < 8,
                    "Dimension index {} exceeds 7",
                    dim_idx
                );
            }
        }

        /// PROPERTY: Anomaly detection returns valid dimension indices.
        #[test]
        fn prop_anomaly_dimensions_valid(
            agent_count in 1usize..5,
            initial_values in arb_kvector_values(),
            update_values in arb_kvector_values(),
            threshold in arb_small_positive(),
        ) {
            let mut service = TemporalKVectorService::new();

            for i in 0..agent_count {
                let did = format!("did:agent:{}", i);
                let initial = make_kvec(initial_values, 1);
                service.register_agent(&did, initial);

                let update = make_kvec_now(update_values);
                let _ = service.update_observation(&did, update);
            }

            let anomalies = service.detect_anomalies(threshold);

            for (_did, dims) in &anomalies {
                for dim in dims {
                    prop_assert!(
                        *dim < 8,
                        "Anomaly dimension index {} exceeds 7",
                        dim
                    );
                }
            }
        }
    }
}
