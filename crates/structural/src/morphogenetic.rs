//! # Morphogenetic Engine -- Primitive [19]
//!
//! Struggle-driven structural emergence.
//!
//! Morphogenetic fields guide how new structures form in the network.  Unlike
//! biological morphogenetic fields which emerge from gene expression gradients,
//! these fields emerge from **resistance and difficulty** -- struggle is the
//! signal that structure needs to change.
//!
//! Each field has a type (Attracting, Repelling, Guiding, Containing), a
//! strength that measures how much influence it exerts, and a gradient vector
//! that indicates the direction of structural change at any position.
//!
//! Fields **decay** over time.  Only fields sustained by ongoing struggle
//! persist.  This ensures that structural emergence is responsive to current
//! conditions rather than historical artifacts.
//!
//! ## Constitutional Alignment
//!
//! **Evolutionary Progression (Harmony 7)**: Struggle is not a bug; it is the
//! signal that evolution is happening.  Morphogenetic fields channel struggle
//! into productive structural change rather than allowing it to dissipate.
//!
//! ## Three Gates
//!
//! - **Gate 1**: Field `strength` is always in `[0.0, 1.0]`.
//! - **Gate 2**: Warns if a field's strength is decaying toward zero (may need
//!   attention or explicit removal).
//!
//! ## Dependency
//!
//! Depends on [18] Fractal Governance for scale-aware field application.
//!
//! ## Classification
//!
//! E2/N1/M1 -- Privately verifiable / Communal / Temporal.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use living_core::{
    CyclePhase, FieldType, Gate1Check, Gate2Warning, LivingProtocolEvent,
    MorphogeneticField, MorphogeneticFieldUpdatedEvent, EventBus,
};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::error::{LivingResult};

// =============================================================================
// Morphogenetic Engine
// =============================================================================

/// Engine for creating, updating, and decaying morphogenetic fields that guide
/// structural emergence in the network.
pub struct MorphogeneticEngine {
    /// Active fields indexed by field ID.
    fields: HashMap<String, MorphogeneticField>,
    /// Event bus for emitting morphogenetic field events.
    event_bus: Arc<dyn EventBus>,
    /// Whether the engine is active in the current cycle phase.
    active: bool,
}

impl MorphogeneticEngine {
    /// Create a new morphogenetic engine.
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            fields: HashMap::new(),
            event_bus,
            active: false,
        }
    }

    /// Create a new morphogenetic field.
    ///
    /// The field starts with the given initial strength and an empty gradient.
    /// The `source_pattern_id` links back to the governance pattern or
    /// structural element that gave rise to this field.
    pub fn create_field(
        &mut self,
        field_type: FieldType,
        source_pattern_id: String,
        initial_strength: f64,
    ) -> MorphogeneticField {
        let id = Uuid::new_v4().to_string();
        let strength = initial_strength.clamp(0.0, 1.0);
        let now = Utc::now();

        let field = MorphogeneticField {
            id: id.clone(),
            field_type,
            strength,
            gradient: Vec::new(),
            source_pattern_id,
            created: now,
        };

        self.fields.insert(id, field.clone());

        tracing::info!(
            field_id = %field.id,
            field_type = ?field.field_type,
            strength = field.strength,
            "Morphogenetic field created. Struggle-driven structural emergence."
        );

        field
    }

    /// Update a field's strength by a delta (can be positive or negative).
    ///
    /// Strength is clamped to `[0.0, 1.0]`.  Emits a
    /// `MorphogeneticFieldUpdated` event.
    pub fn update_field_strength(
        &mut self,
        field_id: &str,
        delta: f64,
    ) -> Option<MorphogeneticFieldUpdatedEvent> {
        let field = self.fields.get_mut(field_id)?;

        let old_strength = field.strength;
        field.strength = (field.strength + delta).clamp(0.0, 1.0);

        let event = MorphogeneticFieldUpdatedEvent {
            field: field.clone(),
            old_strength,
            new_strength: field.strength,
            timestamp: Utc::now(),
        };

        self.event_bus.publish(LivingProtocolEvent::MorphogeneticFieldUpdated(
            event.clone(),
        ));

        tracing::debug!(
            field_id = %field_id,
            old_strength = old_strength,
            new_strength = field.strength,
            delta = delta,
            "Morphogenetic field strength updated."
        );

        Some(event)
    }

    /// Compute the gradient of a field at a given position.
    ///
    /// The gradient indicates the direction and magnitude of the field's
    /// influence at the specified position.  For the current implementation:
    ///
    /// - **Attracting** fields pull toward the origin (negative gradient).
    /// - **Repelling** fields push away from the origin (positive gradient).
    /// - **Guiding** fields point in a fixed direction scaled by strength.
    /// - **Containing** fields push inward when position exceeds unit boundary.
    pub fn compute_gradient(
        &self,
        field_id: &str,
        position: &[f64],
    ) -> Vec<f64> {
        let field = match self.fields.get(field_id) {
            Some(f) => f,
            None => return vec![0.0; position.len()],
        };

        let strength = field.strength;
        let dims = position.len();

        match field.field_type {
            FieldType::Attracting => {
                // Pull toward origin: gradient = -strength * position / |position|
                let magnitude: f64 = position.iter().map(|v| v * v).sum::<f64>().sqrt();
                if magnitude < f64::EPSILON {
                    vec![0.0; dims]
                } else {
                    position
                        .iter()
                        .map(|v| -strength * v / magnitude)
                        .collect()
                }
            }
            FieldType::Repelling => {
                // Push away from origin: gradient = strength * position / |position|
                let magnitude: f64 = position.iter().map(|v| v * v).sum::<f64>().sqrt();
                if magnitude < f64::EPSILON {
                    vec![0.0; dims]
                } else {
                    position
                        .iter()
                        .map(|v| strength * v / magnitude)
                        .collect()
                }
            }
            FieldType::Guiding => {
                // Fixed direction: gradient proportional to strength along each axis
                // Use the existing gradient if available, otherwise uniform
                if field.gradient.len() == dims {
                    field.gradient.iter().map(|g| g * strength).collect()
                } else {
                    vec![strength / (dims as f64).sqrt(); dims]
                }
            }
            FieldType::Containing => {
                // Push inward when outside unit sphere
                let magnitude: f64 = position.iter().map(|v| v * v).sum::<f64>().sqrt();
                if magnitude <= 1.0 {
                    vec![0.0; dims]
                } else {
                    // Push back toward the boundary
                    let excess = magnitude - 1.0;
                    position
                        .iter()
                        .map(|v| -strength * excess * v / magnitude)
                        .collect()
                }
            }
        }
    }

    /// Apply a field's influence to a mutable JSON structure.
    ///
    /// This is a simple demonstration of field-guided structural emergence.
    /// If the structure has a "strength" key, it is adjusted by the field's
    /// gradient magnitude.  Returns true if the structure was modified.
    pub fn apply_field_to_structure(
        &self,
        field_id: &str,
        structure: &mut serde_json::Value,
    ) -> bool {
        let field = match self.fields.get(field_id) {
            Some(f) => f,
            None => return false,
        };

        // Simple application: modify a "strength" property in the JSON
        if let Some(obj) = structure.as_object_mut() {
            if let Some(val) = obj.get_mut("strength") {
                if let Some(current) = val.as_f64() {
                    let adjustment = match field.field_type {
                        FieldType::Attracting => field.strength * 0.1,
                        FieldType::Repelling => -field.strength * 0.1,
                        FieldType::Guiding => field.strength * 0.05,
                        FieldType::Containing => 0.0,
                    };
                    *val = serde_json::Value::from(current + adjustment);
                    return true;
                }
            }

            // If no "strength" key, add field metadata
            obj.insert(
                "morphogenetic_field".to_string(),
                serde_json::json!({
                    "field_id": field.id,
                    "field_type": format!("{:?}", field.field_type),
                    "strength": field.strength,
                }),
            );
            return true;
        }

        false
    }

    /// Get all currently active (non-zero strength) fields.
    pub fn get_active_fields(&self) -> Vec<&MorphogeneticField> {
        self.fields.values().filter(|f| f.strength > 0.0).collect()
    }

    /// Decay all fields by the given rate.
    ///
    /// Each field's strength is multiplied by `(1.0 - rate)`.  Fields whose
    /// strength drops below `f64::EPSILON` are removed entirely.  Returns the
    /// IDs of removed fields.
    ///
    /// This ensures only struggle-sustained fields persist -- fields that
    /// are not continually reinforced will naturally fade.
    pub fn decay_fields(&mut self, rate: f64) -> Vec<String> {
        let rate = rate.clamp(0.0, 1.0);
        let mut removed = Vec::new();

        // Threshold for removal: fields below this are considered effectively zero.
        // Using 1e-10 rather than f64::EPSILON for practical decay convergence.
        const REMOVAL_THRESHOLD: f64 = 1e-10;

        for (id, field) in &mut self.fields {
            field.strength *= 1.0 - rate;
            if field.strength < REMOVAL_THRESHOLD {
                field.strength = 0.0;
                removed.push(id.clone());
            }
        }

        for id in &removed {
            self.fields.remove(id);
            tracing::info!(
                field_id = %id,
                "Morphogenetic field decayed to zero and removed. \
                 Only struggle-sustained fields persist."
            );
        }

        removed
    }

    /// Get a field by its ID.
    pub fn get_field(&self, field_id: &str) -> Option<&MorphogeneticField> {
        self.fields.get(field_id)
    }

    /// Get the total number of fields (including zero-strength).
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for MorphogeneticEngine {
    fn primitive_id(&self) -> &str {
        "morphogenetic_fields"
    }

    fn primitive_number(&self) -> u8 {
        19
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
        // Morphogenetic fields are active during Co-Creation (structure forms)
        // and Composting (old structures decay).
        self.active = matches!(new_phase, CyclePhase::CoCreation | CyclePhase::Composting);

        // Decay fields at each phase transition to simulate natural fade
        if !self.active {
            self.decay_fields(0.05);
        }

        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        for field in self.fields.values() {
            // Gate 1: strength always in [0.0, 1.0]
            let in_bounds = field.strength >= 0.0 && field.strength <= 1.0;
            checks.push(Gate1Check {
                invariant: format!(
                    "strength in [0.0, 1.0] for field {}",
                    field.id
                ),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some(format!("strength = {}", field.strength))
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        for field in self.fields.values() {
            // Gate 2: warn if a field is near zero strength
            if field.strength > 0.0 && field.strength < 0.05 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Evolutionary Progression (Harmony 7)".to_string(),
                    severity: 0.1,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Field {} has strength {:.4}, approaching zero. \
                         It will decay away unless reinforced by ongoing struggle.",
                        field.id, field.strength
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        matches!(phase, CyclePhase::CoCreation | CyclePhase::Composting)
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let active_count = self.get_active_fields().len();
        let avg_strength = if self.fields.is_empty() {
            0.0
        } else {
            self.fields.values().map(|f| f.strength).sum::<f64>() / self.fields.len() as f64
        };

        serde_json::json!({
            "primitive": "morphogenetic_fields",
            "primitive_number": 19,
            "total_fields": self.fields.len(),
            "active_fields": active_count,
            "average_strength": avg_strength,
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

    fn make_engine() -> MorphogeneticEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        MorphogeneticEngine::new(bus)
    }

    fn make_engine_with_bus() -> (MorphogeneticEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = MorphogeneticEngine::new(bus.clone());
        (engine, bus)
    }

    #[test]
    fn test_create_field() {
        let mut engine = make_engine();
        let field = engine.create_field(
            FieldType::Attracting,
            "pattern-123".to_string(),
            0.8,
        );

        assert_eq!(field.field_type, FieldType::Attracting);
        assert_eq!(field.strength, 0.8);
        assert_eq!(field.source_pattern_id, "pattern-123");
        assert!(field.gradient.is_empty());
        assert_eq!(engine.field_count(), 1);
    }

    #[test]
    fn test_create_field_clamps_strength() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Guiding, "p1".to_string(), 1.5);
        assert_eq!(field.strength, 1.0);

        let field2 = engine.create_field(FieldType::Guiding, "p2".to_string(), -0.3);
        assert_eq!(field2.strength, 0.0);
    }

    #[test]
    fn test_update_field_strength() {
        let (mut engine, bus) = make_engine_with_bus();
        let field = engine.create_field(FieldType::Attracting, "p1".to_string(), 0.5);

        let event = engine.update_field_strength(&field.id, 0.2).unwrap();
        assert_eq!(event.old_strength, 0.5);
        assert!((event.new_strength - 0.7).abs() < f64::EPSILON);
        assert_eq!(bus.event_count(), 1);

        // Verify the field was actually updated
        let updated = engine.get_field(&field.id).unwrap();
        assert!((updated.strength - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_field_strength_clamps() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Repelling, "p1".to_string(), 0.9);

        let event = engine.update_field_strength(&field.id, 0.5).unwrap();
        assert_eq!(event.new_strength, 1.0);

        let event2 = engine.update_field_strength(&field.id, -2.0).unwrap();
        assert_eq!(event2.new_strength, 0.0);
    }

    #[test]
    fn test_update_nonexistent_field() {
        let mut engine = make_engine();
        assert!(engine.update_field_strength("nonexistent", 0.1).is_none());
    }

    #[test]
    fn test_compute_gradient_attracting() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Attracting, "p1".to_string(), 1.0);

        let position = vec![3.0, 4.0]; // magnitude = 5
        let gradient = engine.compute_gradient(&field.id, &position);

        // Should pull toward origin: gradient = -1.0 * position / |position|
        assert!((gradient[0] - (-0.6)).abs() < 1e-10);
        assert!((gradient[1] - (-0.8)).abs() < 1e-10);
    }

    #[test]
    fn test_compute_gradient_repelling() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Repelling, "p1".to_string(), 1.0);

        let position = vec![3.0, 4.0];
        let gradient = engine.compute_gradient(&field.id, &position);

        // Should push away: gradient = 1.0 * position / |position|
        assert!((gradient[0] - 0.6).abs() < 1e-10);
        assert!((gradient[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_compute_gradient_containing_inside() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Containing, "p1".to_string(), 1.0);

        // Position inside unit sphere
        let position = vec![0.3, 0.4];
        let gradient = engine.compute_gradient(&field.id, &position);

        // Inside boundary: no force
        assert!(gradient.iter().all(|g| g.abs() < f64::EPSILON));
    }

    #[test]
    fn test_compute_gradient_containing_outside() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Containing, "p1".to_string(), 1.0);

        // Position outside unit sphere
        let position = vec![3.0, 4.0]; // magnitude = 5, excess = 4
        let gradient = engine.compute_gradient(&field.id, &position);

        // Should push inward
        assert!(gradient[0] < 0.0);
        assert!(gradient[1] < 0.0);
    }

    #[test]
    fn test_compute_gradient_at_origin() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Attracting, "p1".to_string(), 1.0);

        let position = vec![0.0, 0.0];
        let gradient = engine.compute_gradient(&field.id, &position);

        // At origin: zero gradient (no direction to pull)
        assert!(gradient.iter().all(|g| g.abs() < f64::EPSILON));
    }

    #[test]
    fn test_compute_gradient_nonexistent_field() {
        let engine = make_engine();
        let gradient = engine.compute_gradient("nonexistent", &[1.0, 2.0]);
        assert_eq!(gradient, vec![0.0, 0.0]);
    }

    #[test]
    fn test_apply_field_to_structure_with_strength() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Attracting, "p1".to_string(), 0.8);

        let mut structure = serde_json::json!({ "strength": 0.5 });
        let modified = engine.apply_field_to_structure(&field.id, &mut structure);

        assert!(modified);
        let new_strength = structure["strength"].as_f64().unwrap();
        // Attracting adds strength * 0.1 = 0.08
        assert!((new_strength - 0.58).abs() < 1e-10);
    }

    #[test]
    fn test_apply_field_to_structure_without_strength() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Guiding, "p1".to_string(), 0.6);

        let mut structure = serde_json::json!({ "name": "test" });
        let modified = engine.apply_field_to_structure(&field.id, &mut structure);

        assert!(modified);
        assert!(structure["morphogenetic_field"].is_object());
    }

    #[test]
    fn test_apply_field_nonexistent() {
        let engine = make_engine();
        let mut structure = serde_json::json!({ "strength": 0.5 });
        assert!(!engine.apply_field_to_structure("nonexistent", &mut structure));
    }

    #[test]
    fn test_get_active_fields() {
        let mut engine = make_engine();
        engine.create_field(FieldType::Attracting, "p1".to_string(), 0.8);
        engine.create_field(FieldType::Repelling, "p2".to_string(), 0.0);
        engine.create_field(FieldType::Guiding, "p3".to_string(), 0.3);

        let active = engine.get_active_fields();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_decay_fields() {
        let mut engine = make_engine();
        engine.create_field(FieldType::Attracting, "p1".to_string(), 0.8);
        let weak = engine.create_field(FieldType::Repelling, "p2".to_string(), 1e-9);

        // Decay by 50% -- the very weak field (1e-9 * 0.5 = 5e-10 < 1e-10 threshold)
        // should be removed, but the strong field should survive
        let removed = engine.decay_fields(0.99);
        assert!(removed.contains(&weak.id));
        assert_eq!(engine.field_count(), 1);

        // The strong field should have decayed but survived
        let remaining = engine.get_active_fields();
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].strength < 0.8);
    }

    #[test]
    fn test_decay_fields_rate_clamped() {
        let mut engine = make_engine();
        engine.create_field(FieldType::Attracting, "p1".to_string(), 0.5);

        // Rate above 1.0 is clamped to 1.0 (full decay)
        let removed = engine.decay_fields(2.0);
        assert_eq!(removed.len(), 1);
        assert_eq!(engine.field_count(), 0);
    }

    #[test]
    fn test_gate1_passes_normal() {
        let mut engine = make_engine();
        engine.create_field(FieldType::Attracting, "p1".to_string(), 0.5);
        engine.create_field(FieldType::Guiding, "p2".to_string(), 1.0);

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate2_warns_near_zero() {
        let mut engine = make_engine();
        let field = engine.create_field(FieldType::Attracting, "p1".to_string(), 0.5);

        // Decay to near zero
        engine.update_field_strength(&field.id, -0.47);

        let warnings = engine.gate2_check();
        assert!(!warnings.is_empty());
        assert!(warnings[0].reasoning.contains("approaching zero"));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "morphogenetic_fields");
        assert_eq!(engine.primitive_number(), 19);
        assert_eq!(engine.module(), PrimitiveModule::Structural);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(engine.is_active_in_phase(CyclePhase::Composting));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::Kenosis));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine.create_field(FieldType::Attracting, "p1".to_string(), 0.8);
        engine.create_field(FieldType::Repelling, "p2".to_string(), 0.4);

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_fields"], 2);
        assert_eq!(metrics["active_fields"], 2);
        assert_eq!(metrics["primitive_number"], 19);
        let avg = metrics["average_strength"].as_f64().unwrap();
        assert!((avg - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_struggle_driven_field_lifecycle() {
        // End-to-end test: field is created from struggle, reinforced, then
        // decays when struggle ceases.
        let mut engine = make_engine();

        // Struggle emerges -> field created
        let field = engine.create_field(FieldType::Guiding, "struggle-1".to_string(), 0.3);

        // Ongoing struggle reinforces the field
        engine.update_field_strength(&field.id, 0.2);
        engine.update_field_strength(&field.id, 0.1);
        let reinforced = engine.get_field(&field.id).unwrap();
        assert!((reinforced.strength - 0.6).abs() < f64::EPSILON);

        // Struggle ceases -> decay over multiple rounds
        // 0.6 * (1 - 0.1)^n < 1e-10 requires n > log(1e-10/0.6) / log(0.9) ~ 217
        for _ in 0..250 {
            engine.decay_fields(0.1);
        }

        // Field should eventually be removed
        assert!(engine.get_field(&field.id).is_none());
    }
}
