//! # [15] Liminality
//!
//! State machine for identity transitions.  When an entity (agent, DAO,
//! protocol, or community) undergoes a fundamental identity change, it enters
//! a liminal state where it CANNOT be prematurely recategorized.  This
//! protects entities during vulnerable transitions and preserves the
//! constitutional Right to Fair Recourse.
//!
//! ## Phase Progression (forward-only)
//!
//! ```text
//! PreLiminal --> Liminal --> PostLiminal --> Integrated
//! ```
//!
//! No backward transitions are permitted.  An entity may remain in any phase
//! indefinitely -- there is no timeout on identity transformation.
//!
//! ## Epistemic Classification
//!
//! E2 (Privately Verifiable) / N2 (Network Consensus) / M1 (Temporal)
//!
//! ## Metabolism Cycle
//!
//! Active during the **Liminal** phase (3 days) of the 28-day cycle.
//!
//! ## Constitutional Alignment
//!
//! Preserves the Right to Fair Recourse: entities in transition cannot be
//! judged, evicted, or recategorized based on their pre-transition or
//! mid-transition identity.

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use living_core::{
    CyclePhase, Did, EpistemicClassification, EpistemicTier, Gate1Check, Gate2Warning,
    LiminalEntityType, LiminalPhase, LiminalRecord, LiminalTransitionCompletedEvent,
    LiminalTransitionStartedEvent, LivingPrimitive, LivingProtocolError, LivingProtocolEvent,
    LivingResult, MaterialityTier, NormativeTier, PrimitiveModule,
};

// =============================================================================
// Liminality Engine
// =============================================================================

/// Engine for managing liminal (threshold) identity transitions.
///
/// Key constraint: entities in liminal state CANNOT be prematurely
/// recategorized.  The `recategorization_blocked` flag on `LiminalRecord`
/// is set to `true` for all phases except `Integrated`.
pub struct LiminalityEngine {
    /// Active liminal records, keyed by record ID.
    records: HashMap<String, LiminalRecord>,
}

impl LiminalityEngine {
    /// Create a new engine.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Enter a liminal state for the given entity.
    ///
    /// Creates a new `LiminalRecord` in the `PreLiminal` phase with
    /// `recategorization_blocked = true`.
    pub fn enter_liminal_state(
        &mut self,
        entity_did: &Did,
        entity_type: LiminalEntityType,
        previous_identity: Option<String>,
    ) -> LiminalTransitionStartedEvent {
        let now = Utc::now();
        let record = LiminalRecord {
            id: Uuid::new_v4().to_string(),
            entity_did: entity_did.clone(),
            entity_type,
            phase: LiminalPhase::PreLiminal,
            entered: now,
            previous_identity,
            emerging_identity: None,
            recategorization_blocked: true,
        };

        let event = LiminalTransitionStartedEvent {
            record: record.clone(),
            timestamp: now,
        };

        self.records.insert(record.id.clone(), record);

        tracing::info!(
            entity_did = %entity_did,
            "[15] Entity entered liminal state (PreLiminal)"
        );

        event
    }

    /// Advance the phase of a liminal record.
    ///
    /// Phase progression is strictly forward-only:
    ///   PreLiminal -> Liminal -> PostLiminal -> Integrated
    ///
    /// Returns the new phase, or an error if the entity is not in a liminal
    /// state or the transition would be backward/invalid.
    pub fn advance_phase(&mut self, record_id: &str) -> LivingResult<LiminalPhase> {
        let record = self
            .records
            .get_mut(record_id)
            .ok_or_else(|| LivingProtocolError::NotInLiminalState(record_id.to_string()))?;

        let next_phase = match &record.phase {
            LiminalPhase::PreLiminal => LiminalPhase::Liminal,
            LiminalPhase::Liminal => LiminalPhase::PostLiminal,
            LiminalPhase::PostLiminal => LiminalPhase::Integrated,
            LiminalPhase::Integrated => {
                // Already fully integrated -- no further advancement.
                return Ok(LiminalPhase::Integrated);
            }
        };

        let old_phase = record.phase.clone();
        record.phase = next_phase.clone();

        // Recategorization is blocked for all phases except Integrated.
        record.recategorization_blocked = next_phase != LiminalPhase::Integrated;

        tracing::info!(
            record_id = %record_id,
            entity_did = %record.entity_did,
            from = ?old_phase,
            to = ?next_phase,
            "[15] Liminal phase advanced"
        );

        Ok(next_phase)
    }

    /// Set the emerging identity for a liminal entity.
    ///
    /// This describes what the entity is becoming.  Can only be set while
    /// the entity is still in a liminal state (not yet Integrated).
    pub fn set_emerging_identity(&mut self, record_id: &str, identity: String) -> LivingResult<()> {
        let record = self
            .records
            .get_mut(record_id)
            .ok_or_else(|| LivingProtocolError::NotInLiminalState(record_id.to_string()))?;

        if record.phase == LiminalPhase::Integrated {
            return Err(LivingProtocolError::NotInLiminalState(
                record.entity_did.clone(),
            ));
        }

        record.emerging_identity = Some(identity);
        Ok(())
    }

    /// Complete the liminal transition.
    ///
    /// Automatically advances the record through any remaining phases to
    /// `Integrated`, lifts the recategorization block, and returns a
    /// completion event.
    pub fn complete_transition(
        &mut self,
        record_id: &str,
    ) -> LivingResult<LiminalTransitionCompletedEvent> {
        let record = self
            .records
            .get_mut(record_id)
            .ok_or_else(|| LivingProtocolError::NotInLiminalState(record_id.to_string()))?;

        // Advance through remaining phases.
        while record.phase != LiminalPhase::Integrated {
            record.phase = match &record.phase {
                LiminalPhase::PreLiminal => LiminalPhase::Liminal,
                LiminalPhase::Liminal => LiminalPhase::PostLiminal,
                LiminalPhase::PostLiminal => LiminalPhase::Integrated,
                LiminalPhase::Integrated => unreachable!(),
            };
        }

        record.recategorization_blocked = false;
        let now = Utc::now();

        let event = LiminalTransitionCompletedEvent {
            record_id: record_id.to_string(),
            entity_did: record.entity_did.clone(),
            new_identity: record.emerging_identity.clone(),
            timestamp: now,
        };

        tracing::info!(
            record_id = %record_id,
            entity_did = %record.entity_did,
            new_identity = ?record.emerging_identity,
            "[15] Liminal transition completed"
        );

        Ok(event)
    }

    /// Check whether a given entity is currently in a liminal state
    /// (any phase except Integrated, or has no record at all).
    pub fn is_in_liminal_state(&self, entity_did: &Did) -> bool {
        self.records
            .values()
            .any(|r| r.entity_did == *entity_did && r.phase != LiminalPhase::Integrated)
    }

    /// Get all entities currently in a liminal state.
    pub fn get_liminal_entities(&self) -> Vec<&LiminalRecord> {
        self.records
            .values()
            .filter(|r| r.phase != LiminalPhase::Integrated)
            .collect()
    }

    /// Check whether recategorization is blocked for a given entity.
    ///
    /// This is the core constitutional protection: entities in liminal state
    /// cannot be prematurely recategorized.
    pub fn is_recategorization_blocked(&self, entity_did: &Did) -> bool {
        self.records
            .values()
            .any(|r| r.entity_did == *entity_did && r.recategorization_blocked)
    }

    /// Get a liminal record by ID.
    pub fn get_record(&self, record_id: &str) -> Option<&LiminalRecord> {
        self.records.get(record_id)
    }

    /// Total number of tracked records (including completed).
    pub fn total_records(&self) -> usize {
        self.records.len()
    }

    /// Epistemic classification for this primitive.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::PrivatelyVerifiable,
            n: NormativeTier::NetworkConsensus,
            m: MaterialityTier::Temporal,
        }
    }
}

impl Default for LiminalityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LivingPrimitive implementation
// =============================================================================

impl LivingPrimitive for LiminalityEngine {
    fn primitive_id(&self) -> &str {
        "liminality"
    }

    fn primitive_number(&self) -> u8 {
        15
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Relational
    }

    fn tier(&self) -> u8 {
        1
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        let mut events = Vec::new();

        if new_phase == CyclePhase::Liminal {
            tracing::info!("[15] Liminal phase active -- identity transitions enabled");
        }

        // When leaving the Liminal phase, emit events for entities still in
        // transition (they continue but we surface them for visibility).
        if new_phase == CyclePhase::NegativeCapability {
            let in_transition = self.get_liminal_entities();
            tracing::info!(
                count = in_transition.len(),
                "[15] Liminal phase ending, {} entities still in transition",
                in_transition.len()
            );
        }

        // Note: we intentionally do NOT force-complete transitions when the
        // Liminal phase ends.  Entities remain in whatever liminal phase they
        // are in until explicitly advanced -- this is the core protection.
        let _ = &mut events; // avoid unused_mut warning

        Ok(events)
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1 invariant: recategorization_blocked must be true for all
        // non-Integrated records.
        let blocking_correct = self.records.values().all(|r| {
            if r.phase != LiminalPhase::Integrated {
                r.recategorization_blocked
            } else {
                !r.recategorization_blocked
            }
        });
        checks.push(Gate1Check {
            invariant: "recategorization_blocking_consistent".to_string(),
            passed: blocking_correct,
            details: if blocking_correct {
                None
            } else {
                Some("Recategorization blocking inconsistent with liminal phase".to_string())
            },
        });

        // Gate 1 invariant: no record has a phase that is impossible given
        // the enum definition (always passes, but documents the invariant).
        checks.push(Gate1Check {
            invariant: "phase_is_valid_enum_variant".to_string(),
            passed: true,
            details: None,
        });

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();
        let now = Utc::now();

        // Constitutional warning: entities that have been in liminal state for
        // a very long time (> 90 days) may need attention.
        for record in self.records.values() {
            if record.phase != LiminalPhase::Integrated {
                let duration = now - record.entered;
                if duration.num_days() > 90 {
                    warnings.push(Gate2Warning {
                        harmony_violated: "Right to Fair Recourse".to_string(),
                        severity: 0.5,
                        reputation_impact: 0.0,
                        reasoning: format!(
                            "Entity {} has been in liminal state for {} days. \
                             Consider whether transition is stalled.",
                            record.entity_did,
                            duration.num_days()
                        ),
                        user_may_proceed: true,
                    });
                }
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Primarily active during Liminal phase, but entities in liminal state
        // persist across all phases.
        matches!(phase, CyclePhase::Liminal)
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let liminal_count = self
            .records
            .values()
            .filter(|r| r.phase != LiminalPhase::Integrated)
            .count();
        let integrated_count = self
            .records
            .values()
            .filter(|r| r.phase == LiminalPhase::Integrated)
            .count();

        let phase_counts: HashMap<String, usize> = {
            let mut m = HashMap::new();
            for record in self.records.values() {
                let key = format!("{:?}", record.phase);
                *m.entry(key).or_insert(0) += 1;
            }
            m
        };

        serde_json::json!({
            "active_liminal_entities": liminal_count,
            "integrated_entities": integrated_count,
            "total_records": self.records.len(),
            "phase_distribution": phase_counts,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> LiminalityEngine {
        LiminalityEngine::new()
    }

    #[test]
    fn test_enter_liminal_state() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(
            &did,
            LiminalEntityType::Agent,
            Some("validator".to_string()),
        );

        assert_eq!(event.record.entity_did, did);
        assert_eq!(event.record.phase, LiminalPhase::PreLiminal);
        assert!(event.record.recategorization_blocked);
        assert_eq!(
            event.record.previous_identity,
            Some("validator".to_string())
        );
        assert!(engine.is_in_liminal_state(&did));
    }

    #[test]
    fn test_phase_progression_forward_only() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        // PreLiminal -> Liminal
        let phase = engine.advance_phase(&record_id).unwrap();
        assert_eq!(phase, LiminalPhase::Liminal);
        assert!(engine.is_recategorization_blocked(&did));

        // Liminal -> PostLiminal
        let phase = engine.advance_phase(&record_id).unwrap();
        assert_eq!(phase, LiminalPhase::PostLiminal);
        assert!(engine.is_recategorization_blocked(&did));

        // PostLiminal -> Integrated
        let phase = engine.advance_phase(&record_id).unwrap();
        assert_eq!(phase, LiminalPhase::Integrated);
        assert!(!engine.is_recategorization_blocked(&did));
        assert!(!engine.is_in_liminal_state(&did));
    }

    #[test]
    fn test_integrated_stays_integrated() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        // Advance to Integrated.
        engine.advance_phase(&record_id).unwrap();
        engine.advance_phase(&record_id).unwrap();
        engine.advance_phase(&record_id).unwrap();

        // Further advancement stays at Integrated.
        let phase = engine.advance_phase(&record_id).unwrap();
        assert_eq!(phase, LiminalPhase::Integrated);
    }

    #[test]
    fn test_recategorization_blocked_during_transition() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        // Not in liminal state yet -- recategorization is not blocked.
        assert!(!engine.is_recategorization_blocked(&did));

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        // PreLiminal -- blocked.
        assert!(engine.is_recategorization_blocked(&did));

        // Liminal -- still blocked.
        engine.advance_phase(&record_id).unwrap();
        assert!(engine.is_recategorization_blocked(&did));

        // PostLiminal -- still blocked.
        engine.advance_phase(&record_id).unwrap();
        assert!(engine.is_recategorization_blocked(&did));

        // Integrated -- unblocked.
        engine.advance_phase(&record_id).unwrap();
        assert!(!engine.is_recategorization_blocked(&did));
    }

    #[test]
    fn test_set_emerging_identity() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        // Can set emerging identity while in liminal state.
        engine
            .set_emerging_identity(&record_id, "community_leader".to_string())
            .unwrap();

        let record = engine.get_record(&record_id).unwrap();
        assert_eq!(
            record.emerging_identity,
            Some("community_leader".to_string())
        );
    }

    #[test]
    fn test_cannot_set_identity_after_integration() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        // Complete transition.
        engine.complete_transition(&record_id).unwrap();

        // Cannot set identity after integration.
        let result = engine.set_emerging_identity(&record_id, "new_role".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_transition() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Agent, None);
        let record_id = event.record.id.clone();

        engine
            .set_emerging_identity(&record_id, "new_identity".to_string())
            .unwrap();

        let completion = engine.complete_transition(&record_id).unwrap();
        assert_eq!(completion.entity_did, did);
        assert_eq!(completion.new_identity, Some("new_identity".to_string()));

        let record = engine.get_record(&record_id).unwrap();
        assert_eq!(record.phase, LiminalPhase::Integrated);
        assert!(!record.recategorization_blocked);
    }

    #[test]
    fn test_complete_transition_from_any_phase() {
        let mut engine = make_engine();
        let did: Did = "did:myc:alice".into();

        let event = engine.enter_liminal_state(&did, LiminalEntityType::Dao, None);
        let record_id = event.record.id.clone();

        // Complete directly from PreLiminal -- should advance through all
        // intermediate phases.
        let completion = engine.complete_transition(&record_id).unwrap();
        assert_eq!(completion.entity_did, did);

        let record = engine.get_record(&record_id).unwrap();
        assert_eq!(record.phase, LiminalPhase::Integrated);
    }

    #[test]
    fn test_get_liminal_entities() {
        let mut engine = make_engine();

        engine.enter_liminal_state(&"did:myc:alice".into(), LiminalEntityType::Agent, None);
        engine.enter_liminal_state(&"did:myc:bob".into(), LiminalEntityType::Dao, None);

        let entities = engine.get_liminal_entities();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_multiple_entities_independent() {
        let mut engine = make_engine();
        let alice: Did = "did:myc:alice".into();
        let bob: Did = "did:myc:bob".into();

        let event_a = engine.enter_liminal_state(&alice, LiminalEntityType::Agent, None);
        let event_b = engine.enter_liminal_state(&bob, LiminalEntityType::Agent, None);

        // Advance alice but not bob.
        engine.advance_phase(&event_a.record.id).unwrap();

        let record_a = engine.get_record(&event_a.record.id).unwrap();
        let record_b = engine.get_record(&event_b.record.id).unwrap();

        assert_eq!(record_a.phase, LiminalPhase::Liminal);
        assert_eq!(record_b.phase, LiminalPhase::PreLiminal);
    }

    #[test]
    fn test_nonexistent_record_error() {
        let mut engine = make_engine();
        let result = engine.advance_phase("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_gate1_invariants() {
        let mut engine = make_engine();
        engine.enter_liminal_state(&"did:myc:alice".into(), LiminalEntityType::Agent, None);

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate1_after_integration() {
        let mut engine = make_engine();
        let event =
            engine.enter_liminal_state(&"did:myc:alice".into(), LiminalEntityType::Agent, None);
        engine.complete_transition(&event.record.id).unwrap();

        let checks = engine.gate1_check();
        assert!(
            checks.iter().all(|c| c.passed),
            "Gate 1 should pass after integration"
        );
    }

    #[test]
    fn test_entity_type_variants() {
        let mut engine = make_engine();

        engine.enter_liminal_state(&"did:myc:agent".into(), LiminalEntityType::Agent, None);
        engine.enter_liminal_state(&"did:myc:dao".into(), LiminalEntityType::Dao, None);
        engine.enter_liminal_state(
            &"did:myc:protocol".into(),
            LiminalEntityType::Protocol,
            None,
        );
        engine.enter_liminal_state(
            &"did:myc:community".into(),
            LiminalEntityType::Community,
            None,
        );

        assert_eq!(engine.get_liminal_entities().len(), 4);
    }

    #[test]
    fn test_classification() {
        let cls = LiminalityEngine::classification();
        assert_eq!(cls.e, EpistemicTier::PrivatelyVerifiable);
        assert_eq!(cls.n, NormativeTier::NetworkConsensus);
        assert_eq!(cls.m, MaterialityTier::Temporal);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::Liminal));
        assert!(!engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
    }
}
