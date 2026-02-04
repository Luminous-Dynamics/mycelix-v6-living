//! # [13] Entangled Pairs
//!
//! Correlation detection between agents.  Entanglement strength DECAYS without
//! continued co-creation -- this is the core invariant.  Two agents must first
//! accumulate a configurable minimum number of co-creation events before an
//! entanglement can form.
//!
//! ## Epistemic Classification
//!
//! E3 (Cryptographically Proven) / N0 (Personal) / M1 (Temporal)
//!
//! ## Dependencies
//!
//! - [5] Temporal K-Vector -- used for correlation detection
//!
//! ## Constitutional Alignment
//!
//! Entanglement is emergent and voluntary.  It cannot be forced and naturally
//! dissolves when agents stop co-creating -- respecting agent autonomy and the
//! organic nature of relationships.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use living_core::{
    CyclePhase, Did, EntangledPair, EntanglementConfig, EntanglementDecayedEvent,
    EntanglementFormedEvent, EpistemicClassification, EpistemicTier, Gate1Check, Gate2Warning,
    LivingPrimitive, LivingProtocolError, LivingProtocolEvent, LivingResult, MaterialityTier,
    NormativeTier, PrimitiveModule,
};

// =============================================================================
// Co-Creation Event
// =============================================================================

/// Record of a co-creation event between two agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoCreationEvent {
    /// First agent in the co-creation pair.
    pub agent_a: Did,
    /// Second agent in the co-creation pair.
    pub agent_b: Did,
    /// Human-readable description of the co-creation activity.
    pub description: String,
    /// When this co-creation occurred.
    pub timestamp: DateTime<Utc>,
    /// Quality score of the co-creation event [0.0, 1.0].
    pub quality_score: f64,
}

// =============================================================================
// Entanglement Engine
// =============================================================================

/// Engine for managing entangled pairs between agents.
///
/// Tracks co-creation history, forms entanglements when thresholds are met,
/// and applies exponential decay to neglected relationships.
pub struct EntanglementEngine {
    /// Active entanglements, keyed by pair ID.
    pairs: HashMap<String, EntangledPair>,
    /// Co-creation history between agent pairs, keyed by sorted DID tuple.
    co_creation_history: HashMap<(Did, Did), Vec<CoCreationEvent>>,
    /// Configuration controlling thresholds and decay.
    config: EntanglementConfig,
}

impl EntanglementEngine {
    /// Create a new engine with the given configuration.
    pub fn new(config: EntanglementConfig) -> Self {
        Self {
            pairs: HashMap::new(),
            co_creation_history: HashMap::new(),
            config,
        }
    }

    /// Canonical key for a pair of agents (sorted to avoid duplicates).
    fn pair_key(a: &Did, b: &Did) -> (Did, Did) {
        if a <= b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        }
    }

    /// Record a co-creation event between two agents.
    ///
    /// Quality score is clamped to [0.0, 1.0].  Returns the recorded event.
    pub fn record_co_creation(
        &mut self,
        agent_a: &Did,
        agent_b: &Did,
        description: &str,
        quality_score: f64,
    ) -> CoCreationEvent {
        let key = Self::pair_key(agent_a, agent_b);
        let event = CoCreationEvent {
            agent_a: agent_a.clone(),
            agent_b: agent_b.clone(),
            description: description.to_string(),
            timestamp: Utc::now(),
            quality_score: quality_score.clamp(0.0, 1.0),
        };
        self.co_creation_history
            .entry(key)
            .or_default()
            .push(event.clone());

        // If already entangled, refresh the last_co_creation timestamp.
        for pair in self.pairs.values_mut() {
            let matches = (pair.agent_a == *agent_a && pair.agent_b == *agent_b)
                || (pair.agent_a == *agent_b && pair.agent_b == *agent_a);
            if matches {
                pair.last_co_creation = Utc::now();
            }
        }

        event
    }

    /// Check whether two agents are currently entangled.
    pub fn check_entanglement(&self, agent_a: &Did, agent_b: &Did) -> Option<&EntangledPair> {
        self.pairs.values().find(|p| {
            (p.agent_a == *agent_a && p.agent_b == *agent_b)
                || (p.agent_a == *agent_b && p.agent_b == *agent_a)
        })
    }

    /// Attempt to form an entanglement between two agents.
    ///
    /// Requires at least `config.min_co_creation_events` co-creation events in
    /// their shared history.  Returns an error if the threshold is not met.
    /// If an entanglement already exists, the existing pair ID is returned in
    /// the event without creating a duplicate.
    pub fn form_entanglement(
        &mut self,
        agent_a: &Did,
        agent_b: &Did,
    ) -> LivingResult<EntanglementFormedEvent> {
        // Check for existing entanglement.
        if let Some(existing) = self.check_entanglement(agent_a, agent_b) {
            return Ok(EntanglementFormedEvent {
                pair: existing.clone(),
                timestamp: Utc::now(),
            });
        }

        // Verify co-creation history meets threshold.
        let key = Self::pair_key(agent_a, agent_b);
        let history = self.co_creation_history.get(&key);
        let event_count = history.map(|h| h.len()).unwrap_or(0);

        if (event_count as u32) < self.config.min_co_creation_events {
            return Err(LivingProtocolError::EntanglementRequiresHistory);
        }

        // Compute initial strength from average quality of co-creation events.
        let avg_quality: f64 = history
            .map(|h| h.iter().map(|e| e.quality_score).sum::<f64>() / h.len() as f64)
            .unwrap_or(0.0);
        let initial_strength = (self.config.base_strength * (1.0 + avg_quality) / 2.0).clamp(0.0, 1.0);

        let now = Utc::now();
        let pair = EntangledPair {
            id: Uuid::new_v4().to_string(),
            agent_a: agent_a.clone(),
            agent_b: agent_b.clone(),
            entanglement_strength: initial_strength,
            formed: now,
            last_co_creation: now,
            decay_rate: self.config.decay_rate_per_day,
        };

        let event = EntanglementFormedEvent {
            pair: pair.clone(),
            timestamp: now,
        };
        self.pairs.insert(pair.id.clone(), pair);

        tracing::info!(
            agent_a = %agent_a,
            agent_b = %agent_b,
            strength = initial_strength,
            "[13] Entanglement formed"
        );

        Ok(event)
    }

    /// Update the recorded entanglement strength based on time decay.
    ///
    /// Returns the current (decayed) strength, or an error if the pair is not
    /// found.
    pub fn update_entanglement(&mut self, pair_id: &str) -> LivingResult<f64> {
        let pair = self.pairs.get_mut(pair_id).ok_or_else(|| {
            LivingProtocolError::AgentNotFound(pair_id.to_string())
        })?;
        let now = Utc::now();
        let current = pair.current_strength(now);
        pair.entanglement_strength = current;
        Ok(current)
    }

    /// Apply decay to all active entanglements, removing any whose strength
    /// falls below `config.min_strength_threshold`.
    ///
    /// Returns a list of decay events for pairs that were removed.
    pub fn decay_all(&mut self, now: DateTime<Utc>) -> Vec<EntanglementDecayedEvent> {
        let threshold = self.config.min_strength_threshold;
        let mut decayed_events = Vec::new();
        let mut to_remove = Vec::new();

        for (id, pair) in &self.pairs {
            let strength = pair.current_strength(now);
            if strength < threshold {
                decayed_events.push(EntanglementDecayedEvent {
                    pair_id: id.clone(),
                    agent_a: pair.agent_a.clone(),
                    agent_b: pair.agent_b.clone(),
                    final_strength: strength,
                    timestamp: now,
                });
                to_remove.push(id.clone());
            }
        }

        for id in &to_remove {
            self.pairs.remove(id);
            tracing::info!(pair_id = %id, "[13] Entanglement decayed and removed");
        }

        decayed_events
    }

    /// List all entangled partners of a given agent, with current strength.
    pub fn get_entangled_partners(&self, agent_did: &Did) -> Vec<(Did, f64)> {
        let now = Utc::now();
        self.pairs
            .values()
            .filter_map(|pair| {
                if pair.agent_a == *agent_did {
                    Some((pair.agent_b.clone(), pair.current_strength(now)))
                } else if pair.agent_b == *agent_did {
                    Some((pair.agent_a.clone(), pair.current_strength(now)))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Number of active entanglements.
    pub fn active_pair_count(&self) -> usize {
        self.pairs.len()
    }

    /// Epistemic classification for this primitive.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::CryptographicallyProven,
            n: NormativeTier::Personal,
            m: MaterialityTier::Temporal,
        }
    }
}

// =============================================================================
// LivingPrimitive implementation
// =============================================================================

impl LivingPrimitive for EntanglementEngine {
    fn primitive_id(&self) -> &str {
        "entangled_pairs"
    }

    fn primitive_number(&self) -> u8 {
        13
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Relational
    }

    fn tier(&self) -> u8 {
        2
    }

    fn on_phase_change(
        &mut self,
        new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        let mut events = Vec::new();

        match new_phase {
            CyclePhase::CoCreation => {
                // During co-creation phase, apply decay to surface neglected
                // entanglements and remove dead ones.
                let now = Utc::now();
                let decayed = self.decay_all(now);
                for d in decayed {
                    events.push(LivingProtocolEvent::EntanglementDecayed(d));
                }
            }
            _ => {}
        }

        Ok(events)
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1 invariant: all entanglement strengths in [0.0, 1.0].
        let all_bounded = self.pairs.values().all(|p| {
            p.entanglement_strength >= 0.0 && p.entanglement_strength <= 1.0
        });
        checks.push(Gate1Check {
            invariant: "entanglement_strength_bounded".to_string(),
            passed: all_bounded,
            details: if all_bounded {
                None
            } else {
                Some("One or more entanglement strengths outside [0.0, 1.0]".to_string())
            },
        });

        // Gate 1 invariant: no self-entanglement.
        let no_self = self.pairs.values().all(|p| p.agent_a != p.agent_b);
        checks.push(Gate1Check {
            invariant: "no_self_entanglement".to_string(),
            passed: no_self,
            details: if no_self {
                None
            } else {
                Some("Self-entanglement detected".to_string())
            },
        });

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();
        let now = Utc::now();

        // Constitutional warning: entanglements near death should be surfaced.
        for pair in self.pairs.values() {
            let strength = pair.current_strength(now);
            if strength < self.config.min_strength_threshold * 3.0 && strength >= self.config.min_strength_threshold {
                warnings.push(Gate2Warning {
                    harmony_violated: "Sacred Reciprocity".to_string(),
                    severity: 0.3,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Entanglement between {} and {} is weakening (strength {:.3}). Co-creation needed.",
                        pair.agent_a, pair.agent_b, strength
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        matches!(phase, CyclePhase::CoCreation | CyclePhase::Eros)
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let now = Utc::now();
        let strengths: Vec<f64> = self
            .pairs
            .values()
            .map(|p| p.current_strength(now))
            .collect();
        let avg_strength = if strengths.is_empty() {
            0.0
        } else {
            strengths.iter().sum::<f64>() / strengths.len() as f64
        };

        serde_json::json!({
            "active_pairs": self.pairs.len(),
            "average_strength": avg_strength,
            "co_creation_pairs_tracked": self.co_creation_history.len(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn default_config() -> EntanglementConfig {
        EntanglementConfig {
            min_co_creation_events: 3,
            base_strength: 0.5,
            decay_rate_per_day: 0.1,
            min_strength_threshold: 0.05,
        }
    }

    fn make_engine() -> EntanglementEngine {
        EntanglementEngine::new(default_config())
    }

    #[test]
    fn test_co_creation_recording() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        let event = engine.record_co_creation(&a, &b, "joint proposal", 0.8);
        assert_eq!(event.agent_a, a);
        assert_eq!(event.agent_b, b);
        assert!((event.quality_score - 0.8).abs() < f64::EPSILON);

        let key = EntanglementEngine::pair_key(&a, &b);
        assert_eq!(engine.co_creation_history[&key].len(), 1);
    }

    #[test]
    fn test_entanglement_requires_history() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        // No co-creation history -> should fail.
        let result = engine.form_entanglement(&a, &b);
        assert!(result.is_err());
        match result.unwrap_err() {
            LivingProtocolError::EntanglementRequiresHistory => {}
            other => panic!("Expected EntanglementRequiresHistory, got: {:?}", other),
        }
    }

    #[test]
    fn test_entanglement_forms_with_sufficient_history() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        // Record 3 co-creation events (matches min_co_creation_events).
        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("event {}", i), 0.7);
        }

        let result = engine.form_entanglement(&a, &b);
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.pair.agent_a, a);
        assert_eq!(event.pair.agent_b, b);
        assert!(event.pair.entanglement_strength > 0.0);
        assert!(event.pair.entanglement_strength <= 1.0);
    }

    #[test]
    fn test_entanglement_idempotent() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("event {}", i), 0.7);
        }

        let event1 = engine.form_entanglement(&a, &b).unwrap();
        let event2 = engine.form_entanglement(&a, &b).unwrap();

        // Same pair ID -- no duplicate created.
        assert_eq!(event1.pair.id, event2.pair.id);
        assert_eq!(engine.active_pair_count(), 1);
    }

    #[test]
    fn test_pair_key_is_symmetric() {
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();
        assert_eq!(
            EntanglementEngine::pair_key(&a, &b),
            EntanglementEngine::pair_key(&b, &a)
        );
    }

    #[test]
    fn test_decay_reduces_strength() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("event {}", i), 0.9);
        }
        engine.form_entanglement(&a, &b).unwrap();

        let pair = engine.check_entanglement(&a, &b).unwrap();
        let initial_strength = pair.entanglement_strength;

        // Check strength after 10 days of no co-creation.
        let future = Utc::now() + Duration::days(10);
        let strength_after = pair.current_strength(future);
        assert!(
            strength_after < initial_strength,
            "Strength should decay: {} should be < {}",
            strength_after,
            initial_strength
        );
    }

    #[test]
    fn test_decay_all_removes_dead_pairs() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("event {}", i), 0.9);
        }
        engine.form_entanglement(&a, &b).unwrap();
        assert_eq!(engine.active_pair_count(), 1);

        // Decay far into the future so strength drops below threshold.
        let far_future = Utc::now() + Duration::days(365);
        let events = engine.decay_all(far_future);

        assert_eq!(events.len(), 1);
        assert_eq!(engine.active_pair_count(), 0);
        assert!(events[0].final_strength < engine.config.min_strength_threshold);
    }

    #[test]
    fn test_get_entangled_partners() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();
        let c: Did = "did:myc:carol".into();

        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("ab-{}", i), 0.8);
            engine.record_co_creation(&a, &c, &format!("ac-{}", i), 0.6);
        }
        engine.form_entanglement(&a, &b).unwrap();
        engine.form_entanglement(&a, &c).unwrap();

        let partners = engine.get_entangled_partners(&a);
        assert_eq!(partners.len(), 2);

        let partner_dids: Vec<&Did> = partners.iter().map(|(d, _)| d).collect();
        assert!(partner_dids.contains(&&b));
        assert!(partner_dids.contains(&&c));
    }

    #[test]
    fn test_co_creation_refreshes_entanglement() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        for i in 0..3 {
            engine.record_co_creation(&a, &b, &format!("event {}", i), 0.9);
        }
        engine.form_entanglement(&a, &b).unwrap();

        // Record more co-creation -- this should refresh last_co_creation.
        engine.record_co_creation(&a, &b, "continued work", 0.85);

        let pair = engine.check_entanglement(&a, &b).unwrap();
        let elapsed = Utc::now() - pair.last_co_creation;
        assert!(
            elapsed.num_seconds() < 2,
            "last_co_creation should be refreshed"
        );
    }

    #[test]
    fn test_quality_score_clamped() {
        let mut engine = make_engine();
        let a: Did = "did:myc:alice".into();
        let b: Did = "did:myc:bob".into();

        let event = engine.record_co_creation(&a, &b, "over quality", 1.5);
        assert!((event.quality_score - 1.0).abs() < f64::EPSILON);

        let event = engine.record_co_creation(&a, &b, "negative quality", -0.5);
        assert!((event.quality_score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gate1_invariants() {
        let engine = make_engine();
        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_classification() {
        let cls = EntanglementEngine::classification();
        assert_eq!(cls.e, EpistemicTier::CryptographicallyProven);
        assert_eq!(cls.n, NormativeTier::Personal);
        assert_eq!(cls.m, MaterialityTier::Temporal);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(engine.is_active_in_phase(CyclePhase::Eros));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::Kenosis));
    }
}
