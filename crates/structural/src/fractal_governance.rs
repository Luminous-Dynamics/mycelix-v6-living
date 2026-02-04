//! # Fractal Governance Engine -- Primitive [18]
//!
//! Self-similar governance patterns at all scales.
//!
//! The key invariant of fractal governance is **structural identity**: a
//! governance pattern MUST be structurally identical at every scale.  When a
//! pattern is replicated from one scale to a child or parent scale, the quorum
//! ratio, supermajority ratio, and decision mechanism are preserved exactly.
//! Only the `scale` field changes.
//!
//! ## Scale Hierarchy
//!
//! Individual -> Team -> Community -> Sector -> Regional -> Global
//!
//! Replication can proceed upward (child to parent) or downward (parent to
//! child).  Each replication emits a `FractalPatternReplicated` event.
//!
//! ## Constitutional Alignment
//!
//! **Subsidiarity (Harmony 5)**: Governance decisions should be made at the
//! smallest effective scale.  Fractal governance enables this by ensuring that
//! the same governance logic works at all levels.
//!
//! ## Three Gates
//!
//! - **Gate 1**: Structural identity -- replicated patterns must have identical
//!   quorum_ratio, supermajority_ratio, and decision_mechanism.
//! - **Gate 2**: Warns on patterns with extreme quorum ratios (< 0.1 or > 0.9).
//!
//! ## Dependency
//!
//! Depends on existing governance from the constitution.
//!
//! ## Classification
//!
//! E2/N2/M0 -- Privately verifiable / Network consensus / Ephemeral.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use living_core::{
    CyclePhase, DecisionMechanism, FractalGovernancePattern, FractalPatternReplicatedEvent,
    Gate1Check, Gate2Warning, GovernanceScale, LivingProtocolEvent, EventBus,
};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::error::{LivingProtocolError, LivingResult};

// =============================================================================
// Fractal Governance Engine
// =============================================================================

/// Engine for creating and replicating fractal governance patterns.
///
/// Ensures structural identity of governance patterns across all scales in the
/// hierarchy (Individual -> Team -> Community -> Sector -> Regional -> Global).
pub struct FractalGovernanceEngine {
    /// Registered governance patterns indexed by pattern ID.
    patterns: HashMap<String, FractalGovernancePattern>,
    /// The canonical scale hierarchy, ordered from smallest to largest.
    scale_hierarchy: Vec<GovernanceScale>,
    /// Event bus for emitting fractal governance events.
    event_bus: Arc<dyn EventBus>,
    /// Whether the engine is active in the current cycle phase.
    active: bool,
}

impl FractalGovernanceEngine {
    /// Create a new fractal governance engine with the canonical scale hierarchy.
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            patterns: HashMap::new(),
            scale_hierarchy: vec![
                GovernanceScale::Individual,
                GovernanceScale::Team,
                GovernanceScale::Community,
                GovernanceScale::Sector,
                GovernanceScale::Regional,
                GovernanceScale::Global,
            ],
            event_bus,
            active: false,
        }
    }

    /// Create a new governance pattern at the given scale.
    ///
    /// The pattern has no parent or children initially; use
    /// `replicate_to_child_scale` or `replicate_to_parent_scale` to propagate.
    pub fn create_pattern(
        &mut self,
        scale: GovernanceScale,
        quorum_ratio: f64,
        supermajority_ratio: f64,
        decision_mechanism: DecisionMechanism,
    ) -> FractalGovernancePattern {
        let id = Uuid::new_v4().to_string();

        let pattern = FractalGovernancePattern {
            id: id.clone(),
            scale,
            parent_pattern_id: None,
            child_patterns: Vec::new(),
            quorum_ratio,
            supermajority_ratio,
            decision_mechanism,
        };

        self.patterns.insert(id, pattern.clone());

        tracing::info!(
            pattern_id = %pattern.id,
            scale = ?pattern.scale,
            "Fractal governance pattern created. Subsidiarity: governance at the smallest effective scale."
        );

        pattern
    }

    /// Replicate a pattern to the next smaller (child) scale.
    ///
    /// Creates a structurally identical pattern one step down the hierarchy.
    /// The parent-child relationship is recorded in both patterns.
    ///
    /// Returns an error if the parent is already at the smallest scale
    /// (Individual).
    pub fn replicate_to_child_scale(
        &mut self,
        parent_id: &str,
    ) -> LivingResult<FractalPatternReplicatedEvent> {
        let parent = self.patterns.get(parent_id).ok_or_else(|| {
            LivingProtocolError::FractalScaleMismatch(format!(
                "Pattern {} not found",
                parent_id
            ))
        })?;

        let parent_idx = self.scale_index(&parent.scale).ok_or_else(|| {
            LivingProtocolError::FractalScaleMismatch(format!(
                "Scale {:?} not in hierarchy",
                parent.scale
            ))
        })?;

        if parent_idx == 0 {
            return Err(LivingProtocolError::FractalScaleMismatch(
                "Cannot replicate below Individual scale".to_string(),
            ));
        }

        let child_scale = self.scale_hierarchy[parent_idx - 1].clone();
        let parent_scale = parent.scale.clone();
        let quorum_ratio = parent.quorum_ratio;
        let supermajority_ratio = parent.supermajority_ratio;
        let decision_mechanism = parent.decision_mechanism.clone();

        let child_id = Uuid::new_v4().to_string();

        let child = FractalGovernancePattern {
            id: child_id.clone(),
            scale: child_scale.clone(),
            parent_pattern_id: Some(parent_id.to_string()),
            child_patterns: Vec::new(),
            quorum_ratio,
            supermajority_ratio,
            decision_mechanism,
        };

        // Update parent to record child
        self.patterns
            .get_mut(parent_id)
            .unwrap()
            .child_patterns
            .push(child_id.clone());

        self.patterns.insert(child_id, child.clone());

        let event = FractalPatternReplicatedEvent {
            pattern: child,
            parent_scale,
            child_scale,
            timestamp: Utc::now(),
        };

        self.event_bus
            .publish(LivingProtocolEvent::FractalPatternReplicated(event.clone()));

        tracing::info!(
            parent_id = %parent_id,
            child_id = %event.pattern.id,
            "Fractal pattern replicated to child scale. Structural identity preserved."
        );

        Ok(event)
    }

    /// Replicate a pattern to the next larger (parent) scale.
    ///
    /// Creates a structurally identical pattern one step up the hierarchy.
    /// The parent-child relationship is recorded in both patterns.
    ///
    /// Returns an error if the child is already at the largest scale (Global).
    pub fn replicate_to_parent_scale(
        &mut self,
        child_id: &str,
    ) -> LivingResult<FractalPatternReplicatedEvent> {
        let child = self.patterns.get(child_id).ok_or_else(|| {
            LivingProtocolError::FractalScaleMismatch(format!(
                "Pattern {} not found",
                child_id
            ))
        })?;

        let child_idx = self.scale_index(&child.scale).ok_or_else(|| {
            LivingProtocolError::FractalScaleMismatch(format!(
                "Scale {:?} not in hierarchy",
                child.scale
            ))
        })?;

        if child_idx >= self.scale_hierarchy.len() - 1 {
            return Err(LivingProtocolError::FractalScaleMismatch(
                "Cannot replicate above Global scale".to_string(),
            ));
        }

        let parent_scale = self.scale_hierarchy[child_idx + 1].clone();
        let child_scale = child.scale.clone();
        let quorum_ratio = child.quorum_ratio;
        let supermajority_ratio = child.supermajority_ratio;
        let decision_mechanism = child.decision_mechanism.clone();

        let parent_id = Uuid::new_v4().to_string();

        let parent = FractalGovernancePattern {
            id: parent_id.clone(),
            scale: parent_scale.clone(),
            parent_pattern_id: None,
            child_patterns: vec![child_id.to_string()],
            quorum_ratio,
            supermajority_ratio,
            decision_mechanism,
        };

        // Update child to record parent
        self.patterns
            .get_mut(child_id)
            .unwrap()
            .parent_pattern_id = Some(parent_id.clone());

        self.patterns.insert(parent_id, parent.clone());

        let event = FractalPatternReplicatedEvent {
            pattern: parent,
            parent_scale,
            child_scale,
            timestamp: Utc::now(),
        };

        self.event_bus
            .publish(LivingProtocolEvent::FractalPatternReplicated(event.clone()));

        tracing::info!(
            child_id = %child_id,
            parent_id = %event.pattern.id,
            "Fractal pattern replicated to parent scale. Structural identity preserved."
        );

        Ok(event)
    }

    /// Verify that two patterns are structurally identical.
    ///
    /// Structural identity means: same quorum_ratio, same supermajority_ratio,
    /// and same decision_mechanism.  The scale is allowed (and expected) to
    /// differ.
    pub fn verify_structural_identity(
        &self,
        pattern_a_id: &str,
        pattern_b_id: &str,
    ) -> bool {
        let a = match self.patterns.get(pattern_a_id) {
            Some(p) => p,
            None => return false,
        };
        let b = match self.patterns.get(pattern_b_id) {
            Some(p) => p,
            None => return false,
        };

        Self::patterns_structurally_identical(a, b)
    }

    /// Get all patterns at a given scale.
    pub fn get_patterns_at_scale(
        &self,
        scale: &GovernanceScale,
    ) -> Vec<&FractalGovernancePattern> {
        self.patterns
            .values()
            .filter(|p| &p.scale == scale)
            .collect()
    }

    /// Get the full hierarchy chain for a pattern.
    ///
    /// Follows parent_pattern_id upward and child_patterns downward to collect
    /// every pattern in the fractal hierarchy containing the given pattern.
    pub fn get_hierarchy(&self, pattern_id: &str) -> Vec<&FractalGovernancePattern> {
        let mut result = Vec::new();

        // Walk up to root
        let mut ancestors = Vec::new();
        let mut current = pattern_id.to_string();
        while let Some(pattern) = self.patterns.get(&current) {
            ancestors.push(pattern);
            match &pattern.parent_pattern_id {
                Some(parent_id) => current = parent_id.clone(),
                None => break,
            }
        }

        // Reverse so root is first
        ancestors.reverse();
        result.extend(ancestors);

        // Walk down from the pattern itself (BFS through children),
        // skipping the pattern itself since it's already in ancestors.
        if let Some(pattern) = self.patterns.get(pattern_id) {
            let mut queue: Vec<&str> = pattern.child_patterns.iter().map(|s| s.as_str()).collect();
            while let Some(child_id) = queue.pop() {
                if let Some(child) = self.patterns.get(child_id) {
                    result.push(child);
                    for grandchild_id in &child.child_patterns {
                        queue.push(grandchild_id);
                    }
                }
            }
        }

        result
    }

    /// Get a pattern by its ID.
    pub fn get_pattern(&self, pattern_id: &str) -> Option<&FractalGovernancePattern> {
        self.patterns.get(pattern_id)
    }

    /// Get the total number of patterns.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Get the index of a scale in the hierarchy.
    fn scale_index(&self, scale: &GovernanceScale) -> Option<usize> {
        self.scale_hierarchy.iter().position(|s| s == scale)
    }

    /// Check structural identity between two patterns.
    fn patterns_structurally_identical(
        a: &FractalGovernancePattern,
        b: &FractalGovernancePattern,
    ) -> bool {
        (a.quorum_ratio - b.quorum_ratio).abs() < f64::EPSILON
            && (a.supermajority_ratio - b.supermajority_ratio).abs() < f64::EPSILON
            && a.decision_mechanism == b.decision_mechanism
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for FractalGovernanceEngine {
    fn primitive_id(&self) -> &str {
        "fractal_governance"
    }

    fn primitive_number(&self) -> u8 {
        18
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Structural
    }

    fn tier(&self) -> u8 {
        2
    }

    fn on_phase_change(
        &mut self,
        new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Fractal governance is active during Co-Creation when governance
        // decisions are made and patterns may be replicated.
        self.active = new_phase == CyclePhase::CoCreation;
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: All patterns in a hierarchy must be structurally identical.
        let visited: Vec<String> = self.patterns.keys().cloned().collect();
        for id in &visited {
            if let Some(pattern) = self.patterns.get(id) {
                for child_id in &pattern.child_patterns {
                    if let Some(child) = self.patterns.get(child_id) {
                        let identical =
                            Self::patterns_structurally_identical(pattern, child);
                        checks.push(Gate1Check {
                            invariant: format!(
                                "structural identity between {} ({:?}) and {} ({:?})",
                                id, pattern.scale, child_id, child.scale
                            ),
                            passed: identical,
                            details: if identical {
                                None
                            } else {
                                Some(format!(
                                    "quorum_ratio: {} vs {}, supermajority_ratio: {} vs {}, \
                                     mechanism: {:?} vs {:?}",
                                    pattern.quorum_ratio,
                                    child.quorum_ratio,
                                    pattern.supermajority_ratio,
                                    child.supermajority_ratio,
                                    pattern.decision_mechanism,
                                    child.decision_mechanism,
                                ))
                            },
                        });
                    }
                }
            }
        }

        // Gate 1: quorum_ratio in [0.0, 1.0]
        for pattern in self.patterns.values() {
            let in_bounds = pattern.quorum_ratio >= 0.0 && pattern.quorum_ratio <= 1.0;
            checks.push(Gate1Check {
                invariant: format!(
                    "quorum_ratio in [0.0, 1.0] for pattern {}",
                    pattern.id
                ),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some(format!("quorum_ratio = {}", pattern.quorum_ratio))
                },
            });
        }

        // Gate 1: supermajority_ratio in [0.0, 1.0]
        for pattern in self.patterns.values() {
            let in_bounds =
                pattern.supermajority_ratio >= 0.0 && pattern.supermajority_ratio <= 1.0;
            checks.push(Gate1Check {
                invariant: format!(
                    "supermajority_ratio in [0.0, 1.0] for pattern {}",
                    pattern.id
                ),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some(format!(
                        "supermajority_ratio = {}",
                        pattern.supermajority_ratio
                    ))
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        for pattern in self.patterns.values() {
            // Gate 2: warn on extreme quorum ratios
            if pattern.quorum_ratio < 0.1 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Subsidiarity (Harmony 5)".to_string(),
                    severity: 0.4,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Pattern {} has quorum_ratio = {:.2}, which is extremely low. \
                         Decisions may not be representative.",
                        pattern.id, pattern.quorum_ratio
                    ),
                    user_may_proceed: true,
                });
            }
            if pattern.quorum_ratio > 0.9 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Subsidiarity (Harmony 5)".to_string(),
                    severity: 0.3,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Pattern {} has quorum_ratio = {:.2}, which is extremely high. \
                         Decisions may be impractical at scale.",
                        pattern.id, pattern.quorum_ratio
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        phase == CyclePhase::CoCreation
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let mut scale_counts = serde_json::Map::new();
        for scale in &self.scale_hierarchy {
            let count = self.get_patterns_at_scale(scale).len();
            scale_counts.insert(format!("{:?}", scale), serde_json::Value::from(count));
        }

        serde_json::json!({
            "primitive": "fractal_governance",
            "primitive_number": 18,
            "total_patterns": self.patterns.len(),
            "patterns_by_scale": scale_counts,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::InMemoryEventBus;

    fn make_engine() -> FractalGovernanceEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        FractalGovernanceEngine::new(bus)
    }

    fn make_engine_with_bus() -> (FractalGovernanceEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = FractalGovernanceEngine::new(bus.clone());
        (engine, bus)
    }

    #[test]
    fn test_create_pattern() {
        let mut engine = make_engine();
        let pattern = engine.create_pattern(
            GovernanceScale::Community,
            0.51,
            0.67,
            DecisionMechanism::Supermajority,
        );

        assert_eq!(pattern.scale, GovernanceScale::Community);
        assert_eq!(pattern.quorum_ratio, 0.51);
        assert_eq!(pattern.supermajority_ratio, 0.67);
        assert_eq!(pattern.decision_mechanism, DecisionMechanism::Supermajority);
        assert!(pattern.parent_pattern_id.is_none());
        assert!(pattern.child_patterns.is_empty());
        assert_eq!(engine.pattern_count(), 1);
    }

    #[test]
    fn test_replicate_to_child_scale() {
        let (mut engine, bus) = make_engine_with_bus();
        let parent = engine.create_pattern(
            GovernanceScale::Community,
            0.51,
            0.67,
            DecisionMechanism::Consensus,
        );

        let event = engine.replicate_to_child_scale(&parent.id).unwrap();
        assert_eq!(event.pattern.scale, GovernanceScale::Team);
        assert_eq!(event.pattern.quorum_ratio, 0.51);
        assert_eq!(event.pattern.supermajority_ratio, 0.67);
        assert_eq!(event.pattern.decision_mechanism, DecisionMechanism::Consensus);
        assert_eq!(event.parent_scale, GovernanceScale::Community);
        assert_eq!(event.child_scale, GovernanceScale::Team);

        // Parent should record the child
        let updated_parent = engine.get_pattern(&parent.id).unwrap();
        assert!(updated_parent.child_patterns.contains(&event.pattern.id));

        assert_eq!(engine.pattern_count(), 2);
        assert_eq!(bus.event_count(), 1);
    }

    #[test]
    fn test_replicate_to_parent_scale() {
        let mut engine = make_engine();
        let child = engine.create_pattern(
            GovernanceScale::Team,
            0.60,
            0.75,
            DecisionMechanism::ReputationWeighted,
        );

        let event = engine.replicate_to_parent_scale(&child.id).unwrap();
        assert_eq!(event.pattern.scale, GovernanceScale::Community);
        assert_eq!(event.pattern.quorum_ratio, 0.60);
        assert_eq!(event.pattern.supermajority_ratio, 0.75);
        assert_eq!(event.pattern.decision_mechanism, DecisionMechanism::ReputationWeighted);

        // Child should record the parent
        let updated_child = engine.get_pattern(&child.id).unwrap();
        assert_eq!(updated_child.parent_pattern_id, Some(event.pattern.id.clone()));
    }

    #[test]
    fn test_cannot_replicate_below_individual() {
        let mut engine = make_engine();
        let pattern = engine.create_pattern(
            GovernanceScale::Individual,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );

        let result = engine.replicate_to_child_scale(&pattern.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_replicate_above_global() {
        let mut engine = make_engine();
        let pattern = engine.create_pattern(
            GovernanceScale::Global,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );

        let result = engine.replicate_to_parent_scale(&pattern.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_structural_identity_verified() {
        let mut engine = make_engine();
        let parent = engine.create_pattern(
            GovernanceScale::Sector,
            0.51,
            0.67,
            DecisionMechanism::Supermajority,
        );

        let event = engine.replicate_to_child_scale(&parent.id).unwrap();
        let child_id = event.pattern.id;

        assert!(engine.verify_structural_identity(&parent.id, &child_id));
    }

    #[test]
    fn test_structural_identity_fails_when_different() {
        let mut engine = make_engine();
        let a = engine.create_pattern(
            GovernanceScale::Community,
            0.51,
            0.67,
            DecisionMechanism::Supermajority,
        );
        let b = engine.create_pattern(
            GovernanceScale::Community,
            0.60,
            0.75,
            DecisionMechanism::Consent,
        );

        assert!(!engine.verify_structural_identity(&a.id, &b.id));
    }

    #[test]
    fn test_get_patterns_at_scale() {
        let mut engine = make_engine();
        engine.create_pattern(
            GovernanceScale::Team,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );
        engine.create_pattern(
            GovernanceScale::Team,
            0.6,
            0.75,
            DecisionMechanism::Consensus,
        );
        engine.create_pattern(
            GovernanceScale::Community,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );

        let team_patterns = engine.get_patterns_at_scale(&GovernanceScale::Team);
        assert_eq!(team_patterns.len(), 2);

        let community_patterns = engine.get_patterns_at_scale(&GovernanceScale::Community);
        assert_eq!(community_patterns.len(), 1);
    }

    #[test]
    fn test_get_hierarchy() {
        let mut engine = make_engine();
        let root = engine.create_pattern(
            GovernanceScale::Regional,
            0.51,
            0.67,
            DecisionMechanism::Supermajority,
        );

        let child_event = engine.replicate_to_child_scale(&root.id).unwrap();
        let _grandchild_event = engine
            .replicate_to_child_scale(&child_event.pattern.id)
            .unwrap();

        let hierarchy = engine.get_hierarchy(&child_event.pattern.id);
        // Should contain: root, child (the query), grandchild
        assert_eq!(hierarchy.len(), 3);
        assert_eq!(hierarchy[0].scale, GovernanceScale::Regional);
        assert_eq!(hierarchy[1].scale, GovernanceScale::Sector);
        assert_eq!(hierarchy[2].scale, GovernanceScale::Community);
    }

    #[test]
    fn test_full_hierarchy_replication() {
        let mut engine = make_engine();
        let global = engine.create_pattern(
            GovernanceScale::Global,
            0.51,
            0.67,
            DecisionMechanism::Supermajority,
        );

        // Replicate all the way down
        let regional_event = engine.replicate_to_child_scale(&global.id).unwrap();
        let sector_event = engine
            .replicate_to_child_scale(&regional_event.pattern.id)
            .unwrap();
        let community_event = engine
            .replicate_to_child_scale(&sector_event.pattern.id)
            .unwrap();
        let team_event = engine
            .replicate_to_child_scale(&community_event.pattern.id)
            .unwrap();
        let individual_event = engine
            .replicate_to_child_scale(&team_event.pattern.id)
            .unwrap();

        assert_eq!(engine.pattern_count(), 6);

        // All patterns should be structurally identical
        assert!(engine.verify_structural_identity(&global.id, &individual_event.pattern.id));
        assert!(engine.verify_structural_identity(
            &regional_event.pattern.id,
            &team_event.pattern.id
        ));
    }

    #[test]
    fn test_gate1_structural_identity() {
        let mut engine = make_engine();
        let parent = engine.create_pattern(
            GovernanceScale::Community,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );
        engine.replicate_to_child_scale(&parent.id).unwrap();

        let checks = engine.gate1_check();
        assert!(
            checks.iter().all(|c| c.passed),
            "All Gate 1 checks should pass after normal replication"
        );
    }

    #[test]
    fn test_gate2_warns_extreme_quorum() {
        let mut engine = make_engine();
        engine.create_pattern(
            GovernanceScale::Community,
            0.05,
            0.67,
            DecisionMechanism::Consent,
        );

        let warnings = engine.gate2_check();
        assert!(!warnings.is_empty());
        assert!(warnings[0].reasoning.contains("extremely low"));
    }

    #[test]
    fn test_nonexistent_pattern_operations() {
        let mut engine = make_engine();
        assert!(engine.replicate_to_child_scale("nonexistent").is_err());
        assert!(engine.replicate_to_parent_scale("nonexistent").is_err());
        assert!(!engine.verify_structural_identity("a", "b"));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "fractal_governance");
        assert_eq!(engine.primitive_number(), 18);
        assert_eq!(engine.module(), PrimitiveModule::Structural);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine.create_pattern(
            GovernanceScale::Community,
            0.5,
            0.67,
            DecisionMechanism::Consent,
        );
        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_patterns"], 1);
        assert_eq!(metrics["primitive_number"], 18);
    }
}
