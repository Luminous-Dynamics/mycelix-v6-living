//! # Composting Engine — Primitive [1]
//!
//! Decomposition of failed entities with nutrient extraction.
//!
//! The composting engine takes failed proposals, abandoned projects, expired claims,
//! deprecated components, and dissolved DAOs and decomposes them into **nutrients** --
//! structured learnings that are published back to the DKG for network benefit.
//!
//! ## Constitutional Alignment
//!
//! **Sacred Reciprocity (Harmony 6)**: Nothing is wasted. Failure is composted into
//! knowledge that strengthens the network. This is the metabolic alternative to
//! simply deleting failed entities.
//!
//! ## Three Gates
//!
//! - **Gate 1**: `decomposition_progress` is always clamped to `[0.0, 1.0]`.
//! - **Gate 2**: Warns if composting a healthy (non-failed) entity.
//!
//! ## Dependency
//!
//! Composting may inherit from failed wound healing: if a wound cannot be healed
//! (agent refuses restitution, abandons process), the wound record itself can be
//! composted to extract learnings about systemic failure modes.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use living_core::{
    CompostableEntity, CompostingRecord, CyclePhase, EntityId,
    EpistemicClassification, Gate1Check, Gate2Warning, LivingProtocolEvent,
    Nutrient, EventBus,
    CompostingStartedEvent, NutrientExtractedEvent, CompostingCompletedEvent,
    CompostingConfig,
};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::error::{LivingProtocolError, LivingResult};

// =============================================================================
// Composting Reason
// =============================================================================

/// Why an entity is being composted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompostingReason {
    /// Proposal failed to reach quorum or was rejected.
    ProposalFailed { vote_count: u64, required: u64 },
    /// Project abandoned by its maintainers.
    ProjectAbandoned { last_activity_days: u64 },
    /// Claim expired without renewal.
    ClaimExpired { original_claim_id: String },
    /// Component deprecated by governance decision.
    ComponentDeprecated { governance_decision_id: String },
    /// DAO dissolved by its members.
    DaoDissolvedByMembers,
    /// Wound healing failed — agent refused restitution.
    WoundHealingFailed { wound_id: String },
    /// Other reason with free-text description.
    Other(String),
}

// =============================================================================
// Composting Engine
// =============================================================================

/// The composting engine manages decomposition of failed entities and
/// extraction of nutrients (learnings) for the network.
pub struct CompostingEngine {
    /// Active composting records indexed by record ID.
    records: HashMap<String, CompostingRecord>,
    /// Configuration.
    config: CompostingConfig,
    /// Event bus for emitting composting events.
    event_bus: Arc<dyn EventBus>,
    /// Reason metadata for each composting record (not stored in core type).
    reasons: HashMap<String, CompostingReason>,
    /// Whether we are in the composting phase of the metabolism cycle.
    active: bool,
}

impl CompostingEngine {
    /// Create a new composting engine with the given configuration and event bus.
    pub fn new(config: CompostingConfig, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            records: HashMap::new(),
            config,
            event_bus,
            reasons: HashMap::new(),
            active: false,
        }
    }

    /// Start composting a failed entity.
    ///
    /// Creates a new `CompostingRecord`, emits a `CompostingStarted` event,
    /// and returns the record. Gate 2 warns if the entity type does not
    /// correspond to an obviously failed entity.
    pub fn start_composting(
        &mut self,
        entity_type: CompostableEntity,
        entity_id: EntityId,
        reason: CompostingReason,
    ) -> LivingResult<CompostingRecord> {
        let record_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Gate 2: warn if composting something that may not be failed
        if self.is_potentially_healthy(&entity_type, &reason) {
            tracing::warn!(
                entity_id = %entity_id,
                entity_type = ?entity_type,
                reason = ?reason,
                "Gate 2 warning: composting an entity that may still be healthy. \
                 Constitutional alignment: Sacred Reciprocity (Harmony 6) — \
                 composting should apply to genuinely failed entities."
            );
        }

        let record = CompostingRecord {
            id: record_id.clone(),
            entity_type: entity_type.clone(),
            entity_id: entity_id.clone(),
            started: now,
            nutrients: Vec::new(),
            // Gate 1: progress starts at 0.0 (always in [0.0, 1.0])
            decomposition_progress: 0.0,
            completed: None,
        };

        self.records.insert(record_id.clone(), record.clone());
        self.reasons.insert(record_id.clone(), reason);

        // Emit event
        self.event_bus.publish(LivingProtocolEvent::CompostingStarted(
            CompostingStartedEvent {
                record_id,
                entity_type,
                entity_id,
                timestamp: now,
            },
        ));

        Ok(record)
    }

    /// Extract a nutrient (learning) from an active composting record.
    ///
    /// Each nutrient is a structured learning with an epistemic classification
    /// that will be published to the DKG for network benefit.
    pub fn extract_nutrient(
        &mut self,
        record_id: &str,
        learning: String,
        classification: EpistemicClassification,
    ) -> LivingResult<Nutrient> {
        let record = self.records.get_mut(record_id).ok_or_else(|| {
            LivingProtocolError::CompostingIneligible(
                record_id.to_string(),
                "Composting record not found".to_string(),
            )
        })?;

        if record.completed.is_some() {
            return Err(LivingProtocolError::CompostingIneligible(
                record_id.to_string(),
                "Composting already completed".to_string(),
            ));
        }

        // Check nutrient limit
        if record.nutrients.len() >= self.config.max_nutrients_per_entity {
            return Err(LivingProtocolError::CompostingIneligible(
                record_id.to_string(),
                format!(
                    "Maximum nutrients ({}) already extracted",
                    self.config.max_nutrients_per_entity
                ),
            ));
        }

        let now = Utc::now();
        let nutrient = Nutrient {
            id: Uuid::new_v4().to_string(),
            source_entity: record.entity_id.clone(),
            learning: learning.clone(),
            classification,
            extracted_at: now,
            published: self.config.auto_publish_nutrients,
        };

        record.nutrients.push(nutrient.clone());

        // Advance decomposition progress proportionally
        // Each nutrient contributes roughly equally to total decomposition
        let max = self.config.max_nutrients_per_entity as f64;
        let current_count = record.nutrients.len() as f64;
        record.decomposition_progress = (current_count / max).clamp(0.0, 1.0);

        // Emit event
        self.event_bus.publish(LivingProtocolEvent::NutrientExtracted(
            NutrientExtractedEvent {
                record_id: record_id.to_string(),
                nutrient: nutrient.clone(),
                timestamp: now,
            },
        ));

        Ok(nutrient)
    }

    /// Complete composting for a record, returning all extracted nutrients.
    ///
    /// Sets `decomposition_progress` to 1.0 and marks the record as completed.
    /// After completion, no further nutrients can be extracted.
    pub fn complete_composting(&mut self, record_id: &str) -> LivingResult<Vec<Nutrient>> {
        let record = self.records.get_mut(record_id).ok_or_else(|| {
            LivingProtocolError::CompostingIneligible(
                record_id.to_string(),
                "Composting record not found".to_string(),
            )
        })?;

        if record.completed.is_some() {
            return Err(LivingProtocolError::CompostingIneligible(
                record_id.to_string(),
                "Composting already completed".to_string(),
            ));
        }

        let now = Utc::now();
        record.decomposition_progress = 1.0;
        record.completed = Some(now);

        let nutrients = record.nutrients.clone();

        // Emit event
        self.event_bus.publish(LivingProtocolEvent::CompostingCompleted(
            CompostingCompletedEvent {
                record_id: record_id.to_string(),
                entity_id: record.entity_id.clone(),
                total_nutrients: nutrients.len(),
                timestamp: now,
            },
        ));

        tracing::info!(
            record_id = %record_id,
            entity_id = %record.entity_id,
            nutrients = nutrients.len(),
            "Composting completed. Sacred Reciprocity: {} learnings returned to the network.",
            nutrients.len()
        );

        Ok(nutrients)
    }

    /// Get all composting records that are still in progress (not completed).
    pub fn get_active_composting(&self) -> Vec<CompostingRecord> {
        self.records
            .values()
            .filter(|r| r.completed.is_none())
            .cloned()
            .collect()
    }

    /// Get a specific composting record by ID.
    pub fn get_record(&self, record_id: &str) -> Option<&CompostingRecord> {
        self.records.get(record_id)
    }

    /// Get the reason for a composting record.
    pub fn get_reason(&self, record_id: &str) -> Option<&CompostingReason> {
        self.reasons.get(record_id)
    }

    /// Get all completed composting records.
    pub fn get_completed_composting(&self) -> Vec<CompostingRecord> {
        self.records
            .values()
            .filter(|r| r.completed.is_some())
            .cloned()
            .collect()
    }

    /// Get total nutrients extracted across all records.
    pub fn total_nutrients_extracted(&self) -> usize {
        self.records.values().map(|r| r.nutrients.len()).sum()
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Gate 2 heuristic: is this entity potentially still healthy?
    fn is_potentially_healthy(
        &self,
        entity_type: &CompostableEntity,
        reason: &CompostingReason,
    ) -> bool {
        // Warn if composting a FailedProposal with an "Other" reason
        // (may indicate healthy entity being composted prematurely)
        matches!(
            (entity_type, reason),
            (_, CompostingReason::Other(_))
        )
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for CompostingEngine {
    fn primitive_id(&self) -> &str {
        "composting"
    }

    fn primitive_number(&self) -> u8 {
        1
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Metabolism
    }

    fn tier(&self) -> u8 {
        2
    }

    fn on_phase_change(
        &mut self,
        new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        self.active = new_phase == CyclePhase::Composting;
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: decomposition_progress always in [0.0, 1.0]
        for record in self.records.values() {
            let in_bounds =
                record.decomposition_progress >= 0.0 && record.decomposition_progress <= 1.0;
            checks.push(Gate1Check {
                invariant: format!(
                    "decomposition_progress in [0.0, 1.0] for record {}",
                    record.id
                ),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some(format!(
                        "decomposition_progress = {} is out of bounds",
                        record.decomposition_progress
                    ))
                },
            });
        }

        // Gate 1: completed records must have progress == 1.0
        for record in self.records.values() {
            if record.completed.is_some() {
                let at_one = (record.decomposition_progress - 1.0).abs() < f64::EPSILON;
                checks.push(Gate1Check {
                    invariant: format!(
                        "completed record {} has progress == 1.0",
                        record.id
                    ),
                    passed: at_one,
                    details: if at_one {
                        None
                    } else {
                        Some(format!(
                            "completed record has progress = {}",
                            record.decomposition_progress
                        ))
                    },
                });
            }
        }

        // Gate 1: nutrient count never exceeds max
        for record in self.records.values() {
            let within_limit = record.nutrients.len() <= self.config.max_nutrients_per_entity;
            checks.push(Gate1Check {
                invariant: format!(
                    "nutrient count <= {} for record {}",
                    self.config.max_nutrients_per_entity, record.id
                ),
                passed: within_limit,
                details: if within_limit {
                    None
                } else {
                    Some(format!(
                        "nutrient count = {} exceeds max {}",
                        record.nutrients.len(),
                        self.config.max_nutrients_per_entity
                    ))
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Gate 2: warn if composting entities with an "Other" reason
        for (record_id, reason) in &self.reasons {
            if let CompostingReason::Other(desc) = reason {
                warnings.push(Gate2Warning {
                    harmony_violated: "Sacred Reciprocity (Harmony 6)".to_string(),
                    severity: 0.3,
                    reputation_impact: -0.01,
                    reasoning: format!(
                        "Composting record {} uses an 'Other' reason: '{}'. \
                         Ensure this entity has genuinely failed before composting.",
                        record_id, desc
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Composting is primarily active during the Composting phase
        // but can accept records during any phase
        phase == CyclePhase::Composting
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "active_composting": self.get_active_composting().len(),
            "completed_composting": self.get_completed_composting().len(),
            "total_nutrients": self.total_nutrients_extracted(),
            "primitive": "composting",
            "primitive_number": 1,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::{
        EpistemicTier, InMemoryEventBus, MaterialityTier, NormativeTier,
    };
    use proptest::prelude::*;

    fn test_classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::Testimonial,
            n: NormativeTier::Communal,
            m: MaterialityTier::Persistent,
        }
    }

    fn make_engine() -> CompostingEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        CompostingEngine::new(CompostingConfig::default(), bus)
    }

    fn make_engine_with_bus() -> (CompostingEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = CompostingEngine::new(CompostingConfig::default(), bus.clone());
        (engine, bus)
    }

    #[test]
    fn test_start_composting_creates_record() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "proposal-123".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 10,
                    required: 20,
                },
            )
            .unwrap();

        assert_eq!(record.entity_id, "proposal-123");
        assert_eq!(record.entity_type, CompostableEntity::FailedProposal);
        assert_eq!(record.decomposition_progress, 0.0);
        assert!(record.completed.is_none());
        assert!(record.nutrients.is_empty());
    }

    #[test]
    fn test_extract_nutrient_advances_progress() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::AbandonedProject,
                "project-xyz".to_string(),
                CompostingReason::ProjectAbandoned {
                    last_activity_days: 90,
                },
            )
            .unwrap();

        let nutrient = engine
            .extract_nutrient(
                &record.id,
                "Teams need clearer milestones".to_string(),
                test_classification(),
            )
            .unwrap();

        assert!(!nutrient.learning.is_empty());
        assert_eq!(nutrient.source_entity, "project-xyz");

        let updated = engine.get_record(&record.id).unwrap();
        assert!(updated.decomposition_progress > 0.0);
        assert!(updated.decomposition_progress <= 1.0);
    }

    #[test]
    fn test_complete_composting_sets_progress_to_one() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::ExpiredClaim,
                "claim-456".to_string(),
                CompostingReason::ClaimExpired {
                    original_claim_id: "old-claim".to_string(),
                },
            )
            .unwrap();

        engine
            .extract_nutrient(
                &record.id,
                "Claims need explicit expiration dates".to_string(),
                test_classification(),
            )
            .unwrap();

        let nutrients = engine.complete_composting(&record.id).unwrap();
        assert_eq!(nutrients.len(), 1);

        let completed = engine.get_record(&record.id).unwrap();
        assert_eq!(completed.decomposition_progress, 1.0);
        assert!(completed.completed.is_some());
    }

    #[test]
    fn test_cannot_extract_from_completed_record() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 5,
                    required: 10,
                },
            )
            .unwrap();

        engine.complete_composting(&record.id).unwrap();

        let result = engine.extract_nutrient(
            &record.id,
            "too late".to_string(),
            test_classification(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_complete_twice() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::DissolvedDao,
                "dao-1".to_string(),
                CompostingReason::DaoDissolvedByMembers,
            )
            .unwrap();

        engine.complete_composting(&record.id).unwrap();
        let result = engine.complete_composting(&record.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_max_nutrients_enforced() {
        let bus = Arc::new(InMemoryEventBus::new());
        let config = CompostingConfig {
            max_nutrients_per_entity: 2,
            ..Default::default()
        };
        let mut engine = CompostingEngine::new(config, bus);

        let record = engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-2".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 1,
                    required: 10,
                },
            )
            .unwrap();

        engine
            .extract_nutrient(&record.id, "learning 1".to_string(), test_classification())
            .unwrap();
        engine
            .extract_nutrient(&record.id, "learning 2".to_string(), test_classification())
            .unwrap();

        let result = engine.extract_nutrient(
            &record.id,
            "learning 3 - should fail".to_string(),
            test_classification(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_active_composting() {
        let mut engine = make_engine();

        let r1 = engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 1,
                    required: 5,
                },
            )
            .unwrap();
        engine
            .start_composting(
                CompostableEntity::AbandonedProject,
                "proj-1".to_string(),
                CompostingReason::ProjectAbandoned {
                    last_activity_days: 60,
                },
            )
            .unwrap();

        assert_eq!(engine.get_active_composting().len(), 2);

        engine.complete_composting(&r1.id).unwrap();
        assert_eq!(engine.get_active_composting().len(), 1);
    }

    #[test]
    fn test_events_emitted() {
        let (mut engine, bus) = make_engine_with_bus();

        let record = engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 2,
                    required: 10,
                },
            )
            .unwrap();

        engine
            .extract_nutrient(
                &record.id,
                "lesson learned".to_string(),
                test_classification(),
            )
            .unwrap();
        engine.complete_composting(&record.id).unwrap();

        assert_eq!(bus.event_count(), 3); // Started + NutrientExtracted + Completed
    }

    #[test]
    fn test_gate1_all_pass_normal_operation() {
        let mut engine = make_engine();

        engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 1,
                    required: 5,
                },
            )
            .unwrap();

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed), "All Gate 1 checks should pass");
    }

    #[test]
    fn test_gate2_warns_on_other_reason() {
        let mut engine = make_engine();

        engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::Other("just because".to_string()),
            )
            .unwrap();

        let warnings = engine.gate2_check();
        assert!(!warnings.is_empty(), "Gate 2 should warn on 'Other' reason");
        assert!(warnings[0].harmony_violated.contains("Sacred Reciprocity"));
    }

    #[test]
    fn test_wound_healing_failed_composting() {
        let mut engine = make_engine();
        let record = engine
            .start_composting(
                CompostableEntity::DeprecatedComponent,
                "wound-record-1".to_string(),
                CompostingReason::WoundHealingFailed {
                    wound_id: "wound-abc".to_string(),
                },
            )
            .unwrap();

        engine
            .extract_nutrient(
                &record.id,
                "Restitution mechanisms need clearer communication".to_string(),
                test_classification(),
            )
            .unwrap();

        let nutrients = engine.complete_composting(&record.id).unwrap();
        assert_eq!(nutrients.len(), 1);
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "composting");
        assert_eq!(engine.primitive_number(), 1);
        assert_eq!(engine.module(), PrimitiveModule::Metabolism);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::Composting));
        assert!(!engine.is_active_in_phase(CyclePhase::Kenosis));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine
            .start_composting(
                CompostableEntity::FailedProposal,
                "p-1".to_string(),
                CompostingReason::ProposalFailed {
                    vote_count: 1,
                    required: 5,
                },
            )
            .unwrap();

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["active_composting"], 1);
        assert_eq!(metrics["completed_composting"], 0);
        assert_eq!(metrics["total_nutrients"], 0);
    }

    // =========================================================================
    // Proptest: Gate 1 invariant — decomposition_progress in [0.0, 1.0]
    // =========================================================================

    proptest! {
        #[test]
        fn prop_decomposition_progress_bounded(
            num_nutrients in 0usize..=15,
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let config = CompostingConfig {
                max_nutrients_per_entity: 10,
                ..Default::default()
            };
            let mut engine = CompostingEngine::new(config, bus);

            let record = engine
                .start_composting(
                    CompostableEntity::FailedProposal,
                    "prop-test".to_string(),
                    CompostingReason::ProposalFailed {
                        vote_count: 0,
                        required: 1,
                    },
                )
                .unwrap();

            for i in 0..num_nutrients {
                let _ = engine.extract_nutrient(
                    &record.id,
                    format!("learning {}", i),
                    EpistemicClassification {
                        e: EpistemicTier::Testimonial,
                        n: NormativeTier::Personal,
                        m: MaterialityTier::Ephemeral,
                    },
                );
            }

            let r = engine.get_record(&record.id).unwrap();
            prop_assert!(
                r.decomposition_progress >= 0.0 && r.decomposition_progress <= 1.0,
                "decomposition_progress = {} out of bounds",
                r.decomposition_progress
            );
        }

        #[test]
        fn prop_gate1_always_passes(
            num_records in 1usize..=10,
            nutrients_per in 0usize..=5,
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let config = CompostingConfig {
                max_nutrients_per_entity: 10,
                ..Default::default()
            };
            let mut engine = CompostingEngine::new(config, bus);

            for r in 0..num_records {
                let record = engine
                    .start_composting(
                        CompostableEntity::FailedProposal,
                        format!("entity-{}", r),
                        CompostingReason::ProposalFailed {
                            vote_count: 0,
                            required: 1,
                        },
                    )
                    .unwrap();

                for n in 0..nutrients_per {
                    let _ = engine.extract_nutrient(
                        &record.id,
                        format!("learning {}-{}", r, n),
                        EpistemicClassification {
                            e: EpistemicTier::Testimonial,
                            n: NormativeTier::Personal,
                            m: MaterialityTier::Ephemeral,
                        },
                    );
                }

                // Complete half of them
                if r % 2 == 0 {
                    let _ = engine.complete_composting(&record.id);
                }
            }

            let checks = engine.gate1_check();
            for check in &checks {
                prop_assert!(
                    check.passed,
                    "Gate 1 check failed: {} — {:?}",
                    check.invariant,
                    check.details
                );
            }
        }
    }
}
