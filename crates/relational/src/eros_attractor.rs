//! # [14] Eros / Attractor Fields
//!
//! Field computation for *complementary* agents.  Unlike similarity matching,
//! Eros detects agents whose K-Vector strengths fill each other's gaps --
//! where one is weak, the other is strong.  This creates attraction toward
//! wholeness through difference rather than echo-chamber reinforcement.
//!
//! ## Epistemic Classification
//!
//! E1 (Testimonial) / N1 (Communal) / M2 (Persistent)
//!
//! ## Feature Flag
//!
//! Behind `tier3-experimental`.
//!
//! ## Dependencies
//!
//! - [5] Temporal K-Vector -- source of agent K-Vector signatures
//! - [6] Field Interference -- interference patterns inform field strength
//!
//! ## Metabolism Cycle
//!
//! Active during the **Eros** phase (4 days) of the 28-day cycle.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use living_core::{
    AttractorFieldComputedEvent, CyclePhase, Did, EpistemicClassification, EpistemicTier,
    FeatureFlags, Gate1Check, Gate2Warning, KVectorSignature, LivingPrimitive,
    LivingProtocolError, LivingProtocolEvent, LivingResult, MaterialityTier, NormativeTier,
    PrimitiveModule,
};

// =============================================================================
// Attractor Field
// =============================================================================

/// A computed attractor field centered on an agent, listing other agents that
/// are drawn to it by complementarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttractorField {
    /// DID of the agent at the center of this field.
    pub center_did: Did,
    /// Agents attracted to the center, with their attraction strength [0.0, 1.0].
    pub attracted_agents: Vec<(Did, f64)>,
    /// Overall field strength -- average attraction across all attracted agents.
    pub field_strength: f64,
    /// When this field was computed.
    pub computed_at: DateTime<Utc>,
}

// =============================================================================
// Eros Attractor Engine
// =============================================================================

/// Engine for computing Eros / Attractor Fields.
///
/// Identifies complementary agents -- pairs where one agent's K-Vector weaknesses
/// are the other's strengths, and vice versa.  This produces attraction toward
/// wholeness through difference rather than similarity.
pub struct ErosAttractorEngine {
    /// Most recently computed attractor fields.
    attractor_fields: Vec<AttractorField>,
    /// Feature flags (used to check tier3-experimental).
    features: FeatureFlags,
}

impl ErosAttractorEngine {
    /// Create a new engine.
    pub fn new(features: FeatureFlags) -> Self {
        Self {
            attractor_fields: Vec::new(),
            features,
        }
    }

    /// Check whether the eros_attractor feature is enabled.
    fn check_enabled(&self) -> LivingResult<()> {
        if !self.features.eros_attractor {
            return Err(LivingProtocolError::FeatureNotEnabled(
                "Eros / Attractor Fields".to_string(),
                "tier3-experimental".to_string(),
            ));
        }
        Ok(())
    }

    /// Compute attractor fields for all agents with known K-Vectors.
    ///
    /// For each agent, finds complementary agents and computes the field
    /// strength.  Returns events for fields whose strength exceeds a minimum
    /// threshold (0.1).
    pub fn compute_attractor_fields(
        &mut self,
        k_vectors: &HashMap<Did, KVectorSignature>,
    ) -> LivingResult<Vec<AttractorFieldComputedEvent>> {
        self.check_enabled()?;

        let dids: Vec<Did> = k_vectors.keys().cloned().collect();
        let mut events = Vec::new();
        let mut fields = Vec::new();
        let now = Utc::now();

        for center_did in &dids {
            let center_kvec = &k_vectors[center_did];
            let mut attracted: Vec<(Did, f64)> = Vec::new();

            for other_did in &dids {
                if other_did == center_did {
                    continue;
                }
                let other_kvec = &k_vectors[other_did];
                let strength = Self::compute_attraction_strength(center_kvec, other_kvec);
                if strength > 0.1 {
                    attracted.push((other_did.clone(), strength));
                }
            }

            // Sort by attraction strength, strongest first.
            attracted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let field_strength = if attracted.is_empty() {
                0.0
            } else {
                attracted.iter().map(|(_, s)| s).sum::<f64>() / attracted.len() as f64
            };

            if field_strength > 0.0 {
                let field = AttractorField {
                    center_did: center_did.clone(),
                    attracted_agents: attracted.clone(),
                    field_strength,
                    computed_at: now,
                };
                fields.push(field);

                events.push(AttractorFieldComputedEvent {
                    field_center: center_did.clone(),
                    attracted_agents: attracted,
                    field_strength,
                    timestamp: now,
                });
            }
        }

        // Sort fields by strength, strongest first.
        fields.sort_by(|a, b| {
            b.field_strength
                .partial_cmp(&a.field_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.attractor_fields = fields;

        tracing::info!(
            field_count = events.len(),
            "[14] Attractor fields computed"
        );

        Ok(events)
    }

    /// Find agents whose K-Vectors are complementary to the given agent.
    ///
    /// Complementary means: where `agent_did` is weak, the partner is strong,
    /// and vice versa.  Returns pairs of (Did, attraction_strength) sorted by
    /// strength descending.
    pub fn find_complementary_agents(
        &self,
        agent_did: &Did,
        k_vectors: &HashMap<Did, KVectorSignature>,
    ) -> LivingResult<Vec<(Did, f64)>> {
        self.check_enabled()?;

        let center_kvec = k_vectors
            .get(agent_did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(agent_did.clone()))?;

        let mut results: Vec<(Did, f64)> = k_vectors
            .iter()
            .filter(|(did, _)| *did != agent_did)
            .map(|(did, kvec)| {
                let strength = Self::compute_attraction_strength(center_kvec, kvec);
                (did.clone(), strength)
            })
            .filter(|(_, strength)| *strength > 0.1)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// Compute the attraction (complementarity) strength between two K-Vectors.
    ///
    /// **This is NOT similarity.**  Complementarity is high when one agent is
    /// strong where the other is weak.  For each dimension, the complement
    /// contribution is `|a_i - b_i|` weighted by the weakness of the weaker
    /// party.  The final score is normalized to [0.0, 1.0].
    ///
    /// Formula per dimension:
    ///   complement_i = |a_i - b_i| * (1.0 - min(a_i, b_i))
    ///
    /// This rewards large differences where the weaker party is genuinely weak,
    /// rather than two agents who are both strong in slightly different ways.
    pub fn compute_attraction_strength(a: &KVectorSignature, b: &KVectorSignature) -> f64 {
        let va = a.as_array();
        let vb = b.as_array();

        let mut total_complement = 0.0;

        for i in 0..KVectorSignature::DIMENSIONS {
            let diff = (va[i] - vb[i]).abs();
            let weakness = 1.0 - va[i].min(vb[i]);
            total_complement += diff * weakness;
        }

        // Normalize: maximum possible is 8 * 1.0 * 1.0 = 8.0
        let normalized = total_complement / KVectorSignature::DIMENSIONS as f64;
        normalized.clamp(0.0, 1.0)
    }

    /// Return the top N strongest attractor fields from the last computation.
    pub fn get_strongest_attractors(&self, top_n: usize) -> Vec<&AttractorField> {
        self.attractor_fields.iter().take(top_n).collect()
    }

    /// Total number of computed attractor fields.
    pub fn field_count(&self) -> usize {
        self.attractor_fields.len()
    }

    /// Epistemic classification for this primitive.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::Testimonial,
            n: NormativeTier::Communal,
            m: MaterialityTier::Persistent,
        }
    }
}

// =============================================================================
// LivingPrimitive implementation
// =============================================================================

impl LivingPrimitive for ErosAttractorEngine {
    fn primitive_id(&self) -> &str {
        "eros_attractor"
    }

    fn primitive_number(&self) -> u8 {
        14
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Relational
    }

    fn tier(&self) -> u8 {
        3
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Attractor field computation is triggered externally during the Eros
        // phase by providing K-Vectors.  No automatic action on phase change.
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1 invariant: all field strengths in [0.0, 1.0].
        let all_bounded = self
            .attractor_fields
            .iter()
            .all(|f| f.field_strength >= 0.0 && f.field_strength <= 1.0);
        checks.push(Gate1Check {
            invariant: "attractor_field_strength_bounded".to_string(),
            passed: all_bounded,
            details: if all_bounded {
                None
            } else {
                Some("One or more attractor field strengths outside [0.0, 1.0]".to_string())
            },
        });

        // Gate 1 invariant: all attraction strengths in [0.0, 1.0].
        let all_attractions_bounded = self.attractor_fields.iter().all(|f| {
            f.attracted_agents
                .iter()
                .all(|(_, s)| *s >= 0.0 && *s <= 1.0)
        });
        checks.push(Gate1Check {
            invariant: "attraction_strength_bounded".to_string(),
            passed: all_attractions_bounded,
            details: if all_attractions_bounded {
                None
            } else {
                Some("One or more attraction strengths outside [0.0, 1.0]".to_string())
            },
        });

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        // No constitutional warnings for attractor fields -- they are
        // informational and non-coercive.
        Vec::new()
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        matches!(phase, CyclePhase::Eros)
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let avg_field_strength = if self.attractor_fields.is_empty() {
            0.0
        } else {
            self.attractor_fields.iter().map(|f| f.field_strength).sum::<f64>()
                / self.attractor_fields.len() as f64
        };

        serde_json::json!({
            "computed_fields": self.attractor_fields.len(),
            "average_field_strength": avg_field_strength,
            "feature_enabled": self.features.eros_attractor,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_kvec(values: [f64; 8]) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now())
    }

    fn enabled_features() -> FeatureFlags {
        FeatureFlags::all_enabled()
    }

    fn disabled_features() -> FeatureFlags {
        FeatureFlags::default() // eros_attractor defaults to false
    }

    #[test]
    fn test_complementarity_not_similarity() {
        // Agent A is strong in first 4 dims, weak in last 4.
        let a = sample_kvec([0.9, 0.9, 0.9, 0.9, 0.1, 0.1, 0.1, 0.8]);
        // Agent B is weak in first 4 dims, strong in last 4.
        let b = sample_kvec([0.1, 0.1, 0.1, 0.1, 0.9, 0.9, 0.9, 0.8]);
        // Agent C is identical to A (high similarity, low complementarity).
        let c = sample_kvec([0.9, 0.9, 0.9, 0.9, 0.1, 0.1, 0.1, 0.8]);

        let complement_ab = ErosAttractorEngine::compute_attraction_strength(&a, &b);
        let complement_ac = ErosAttractorEngine::compute_attraction_strength(&a, &c);

        assert!(
            complement_ab > complement_ac,
            "Complementary pair should have higher attraction ({}) than similar pair ({})",
            complement_ab,
            complement_ac
        );
    }

    #[test]
    fn test_identical_agents_zero_complementarity() {
        let a = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
        let complement = ErosAttractorEngine::compute_attraction_strength(&a, &a);
        assert!(
            complement.abs() < f64::EPSILON,
            "Identical agents should have zero complementarity, got {}",
            complement
        );
    }

    #[test]
    fn test_attraction_strength_bounded() {
        let a = sample_kvec([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = sample_kvec([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

        let strength = ErosAttractorEngine::compute_attraction_strength(&a, &b);
        assert!(strength >= 0.0 && strength <= 1.0, "Strength {} out of [0, 1]", strength);
    }

    #[test]
    fn test_compute_attractor_fields() {
        let mut engine = ErosAttractorEngine::new(enabled_features());

        let mut k_vectors: HashMap<Did, KVectorSignature> = HashMap::new();
        k_vectors.insert(
            "did:myc:alice".to_string(),
            sample_kvec([0.9, 0.8, 0.7, 0.6, 0.1, 0.2, 0.3, 0.8]),
        );
        k_vectors.insert(
            "did:myc:bob".to_string(),
            sample_kvec([0.1, 0.2, 0.3, 0.4, 0.9, 0.8, 0.7, 0.8]),
        );
        k_vectors.insert(
            "did:myc:carol".to_string(),
            sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.8]),
        );

        let events = engine.compute_attractor_fields(&k_vectors).unwrap();
        assert!(!events.is_empty(), "Should produce attractor field events");

        // All field strengths should be bounded.
        for event in &events {
            assert!(event.field_strength >= 0.0 && event.field_strength <= 1.0);
        }
    }

    #[test]
    fn test_find_complementary_agents() {
        let engine = ErosAttractorEngine::new(enabled_features());

        let mut k_vectors: HashMap<Did, KVectorSignature> = HashMap::new();
        let alice_did: Did = "did:myc:alice".to_string();
        k_vectors.insert(
            alice_did.clone(),
            sample_kvec([0.9, 0.9, 0.1, 0.1, 0.5, 0.5, 0.5, 0.8]),
        );
        k_vectors.insert(
            "did:myc:bob".to_string(),
            sample_kvec([0.1, 0.1, 0.9, 0.9, 0.5, 0.5, 0.5, 0.8]),
        );

        let results = engine
            .find_complementary_agents(&alice_did, &k_vectors)
            .unwrap();
        assert!(!results.is_empty(), "Should find complementary agents");
    }

    #[test]
    fn test_feature_flag_gate() {
        let mut engine = ErosAttractorEngine::new(disabled_features());
        let k_vectors: HashMap<Did, KVectorSignature> = HashMap::new();

        let result = engine.compute_attractor_fields(&k_vectors);
        assert!(result.is_err());
        match result.unwrap_err() {
            LivingProtocolError::FeatureNotEnabled(name, flag) => {
                assert_eq!(flag, "tier3-experimental");
                assert!(name.contains("Eros"));
            }
            other => panic!("Expected FeatureNotEnabled, got: {:?}", other),
        }
    }

    #[test]
    fn test_get_strongest_attractors() {
        let mut engine = ErosAttractorEngine::new(enabled_features());

        let mut k_vectors: HashMap<Did, KVectorSignature> = HashMap::new();
        k_vectors.insert(
            "did:myc:a".to_string(),
            sample_kvec([0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.8]),
        );
        k_vectors.insert(
            "did:myc:b".to_string(),
            sample_kvec([0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.8]),
        );
        k_vectors.insert(
            "did:myc:c".to_string(),
            sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.8]),
        );

        engine.compute_attractor_fields(&k_vectors).unwrap();

        let top = engine.get_strongest_attractors(2);
        assert!(top.len() <= 2);
        // Verify sorted by strength descending.
        if top.len() == 2 {
            assert!(top[0].field_strength >= top[1].field_strength);
        }
    }

    #[test]
    fn test_gate1_invariants() {
        let engine = ErosAttractorEngine::new(enabled_features());
        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_classification() {
        let cls = ErosAttractorEngine::classification();
        assert_eq!(cls.e, EpistemicTier::Testimonial);
        assert_eq!(cls.n, NormativeTier::Communal);
        assert_eq!(cls.m, MaterialityTier::Persistent);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = ErosAttractorEngine::new(enabled_features());
        assert!(engine.is_active_in_phase(CyclePhase::Eros));
        assert!(!engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Liminal));
    }

    #[test]
    fn test_empty_k_vectors() {
        let mut engine = ErosAttractorEngine::new(enabled_features());
        let k_vectors: HashMap<Did, KVectorSignature> = HashMap::new();

        let events = engine.compute_attractor_fields(&k_vectors).unwrap();
        assert!(events.is_empty());
        assert_eq!(engine.field_count(), 0);
    }

    #[test]
    fn test_weakness_weighting() {
        // Both agents strong, big difference -> moderate complementarity.
        let a = sample_kvec([0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9]);
        let b = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);

        // One agent truly weak, other strong -> higher complementarity.
        let c = sample_kvec([0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9]);
        let d = sample_kvec([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        let comp_ab = ErosAttractorEngine::compute_attraction_strength(&a, &b);
        let comp_cd = ErosAttractorEngine::compute_attraction_strength(&c, &d);

        assert!(
            comp_cd > comp_ab,
            "True weakness filling ({}) should score higher than moderate difference ({})",
            comp_cd,
            comp_ab
        );
    }
}
