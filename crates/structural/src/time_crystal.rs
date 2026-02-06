//! # Time-Crystal Consensus Engine -- Primitive [20]
//!
//! Periodic consensus with temporal symmetry.
//!
//! Inspired by time crystals in physics (systems that exhibit periodicity in
//! time rather than space), this consensus mechanism organizes validator
//! participation into periodic structures.  The key insight is that consensus
//! periods have a **phase angle** that advances continuously, and validators
//! are selected deterministically based on their position in the phase cycle.
//!
//! This creates a temporally symmetric consensus structure: the same rules
//! apply at every point in the cycle, but different validators are active at
//! different phases, ensuring both fairness and predictability.
//!
//! ## Feature Flag
//!
//! Behind the `tier3-experimental` feature flag.
//!
//! ## Constitutional Alignment
//!
//! **Sovereignty (Harmony 1)**: Deterministic validator selection ensures that
//! no single entity can monopolize consensus participation.  The periodic
//! structure guarantees that all validators participate equally over time.
//!
//! ## Three Gates
//!
//! - **Gate 1**: `phase_angle` is always in `[0.0, 2*PI)`.
//! - **Gate 1**: Period duration must be positive.
//! - **Gate 2**: Warns if validator set is too small (< 4 for BFT assumptions).
//!
//! ## Dependency
//!
//! Depends on [5] Temporal K-Vector and RB-BFT consensus.
//!
//! ## Classification
//!
//! E2/N0/M2 -- Privately verifiable / Personal / Persistent.

use std::sync::Arc;

use chrono::{Duration, Utc};

use living_core::error::{LivingProtocolError, LivingResult};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Did, EventBus, Gate1Check, Gate2Warning, LivingProtocolEvent, TimeCrystalPeriod,
    TimeCrystalPeriodStartedEvent,
};

// =============================================================================
// Time-Crystal Consensus Engine
// =============================================================================

/// Engine for managing time-crystal consensus periods with temporal symmetry.
///
/// Each period has a continuously advancing phase angle.  Validators are
/// selected deterministically based on their position in the phase cycle,
/// ensuring fair and predictable consensus participation.
pub struct TimeCrystalEngine {
    /// The currently active consensus period, if any.
    current_period: Option<TimeCrystalPeriod>,
    /// History of completed periods.
    period_history: Vec<TimeCrystalPeriod>,
    /// Next period ID counter.
    next_period_id: u64,
    /// Event bus for emitting time-crystal events.
    event_bus: Arc<dyn EventBus>,
    /// Whether the engine is active in the current cycle phase.
    active: bool,
}

impl TimeCrystalEngine {
    /// Create a new time-crystal consensus engine.
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            current_period: None,
            period_history: Vec::new(),
            next_period_id: 1,
            event_bus,
            active: false,
        }
    }

    /// Start a new consensus period with the given validators and duration.
    ///
    /// The period begins at phase angle 0.0 and the phase advances toward
    /// 2*PI.  Validators are assigned evenly across the phase space.
    ///
    /// Returns an error if a period is already in progress.
    pub fn start_period(
        &mut self,
        validators: Vec<Did>,
        duration: Duration,
    ) -> LivingResult<TimeCrystalPeriodStartedEvent> {
        if self.current_period.is_some() {
            return Err(LivingProtocolError::TimeCrystalPeriodViolation(
                "A period is already in progress. Complete it before starting a new one."
                    .to_string(),
            ));
        }

        if validators.is_empty() {
            return Err(LivingProtocolError::TimeCrystalPeriodViolation(
                "At least one validator is required".to_string(),
            ));
        }

        let now = Utc::now();
        let period_id = self.next_period_id;
        self.next_period_id += 1;

        let period = TimeCrystalPeriod {
            period_id,
            phase_angle: 0.0,
            symmetry_group: format!("C{}", validators.len()),
            validators: validators.clone(),
            started: now,
            period_duration: duration,
        };

        self.current_period = Some(period.clone());

        let event = TimeCrystalPeriodStartedEvent {
            period: period.clone(),
            timestamp: now,
        };

        self.event_bus
            .publish(LivingProtocolEvent::TimeCrystalPeriodStarted(event.clone()));

        tracing::info!(
            period_id = period_id,
            validators = validators.len(),
            symmetry_group = %period.symmetry_group,
            "Time-crystal consensus period started. Temporal symmetry: \
             all validators participate equally over the cycle."
        );

        Ok(event)
    }

    /// Advance the phase angle within the current period.
    ///
    /// The phase angle wraps at 2*PI to maintain periodicity.  Returns the
    /// new phase angle.
    ///
    /// Returns an error if no period is active.
    pub fn advance_phase_angle(&mut self, delta: f64) -> LivingResult<f64> {
        let period = self.current_period.as_mut().ok_or_else(|| {
            LivingProtocolError::TimeCrystalPeriodViolation(
                "No active period to advance".to_string(),
            )
        })?;

        let two_pi = 2.0 * std::f64::consts::PI;
        period.phase_angle = (period.phase_angle + delta) % two_pi;

        // Ensure non-negative after modulo
        if period.phase_angle < 0.0 {
            period.phase_angle += two_pi;
        }

        tracing::trace!(
            period_id = period.period_id,
            phase_angle = period.phase_angle,
            "Phase angle advanced."
        );

        Ok(period.phase_angle)
    }

    /// Check whether the current period's elapsed time exceeds its duration.
    pub fn is_period_complete(&self) -> bool {
        match &self.current_period {
            Some(period) => {
                let elapsed = Utc::now() - period.started;
                elapsed >= period.period_duration
            }
            None => false,
        }
    }

    /// Complete the current period, moving it to history.
    ///
    /// Returns the completed period.  Returns an error if no period is active.
    pub fn complete_period(&mut self) -> LivingResult<TimeCrystalPeriod> {
        let period = self.current_period.take().ok_or_else(|| {
            LivingProtocolError::TimeCrystalPeriodViolation(
                "No active period to complete".to_string(),
            )
        })?;

        self.period_history.push(period.clone());

        tracing::info!(
            period_id = period.period_id,
            final_phase_angle = period.phase_angle,
            "Time-crystal period completed. Period archived to history."
        );

        Ok(period)
    }

    /// Get the validator responsible for a given phase angle.
    ///
    /// Validators are evenly distributed across the `[0, 2*PI)` phase space.
    /// The validator for a given angle is determined by which sector the angle
    /// falls in.
    ///
    /// Returns None if no period is active or the validator set is empty.
    pub fn get_validator_for_phase(&self, phase_angle: f64) -> Option<Did> {
        let period = self.current_period.as_ref()?;
        if period.validators.is_empty() {
            return None;
        }

        let two_pi = 2.0 * std::f64::consts::PI;
        let normalized = ((phase_angle % two_pi) + two_pi) % two_pi;
        let sector_size = two_pi / period.validators.len() as f64;
        let index = (normalized / sector_size) as usize;
        let index = index.min(period.validators.len() - 1);

        Some(period.validators[index].clone())
    }

    /// Verify that the temporal symmetry of the current period is maintained.
    ///
    /// Temporal symmetry requires:
    /// 1. Phase angle is in `[0, 2*PI)`.
    /// 2. All validators are distinct.
    /// 3. The symmetry group matches the validator count.
    ///
    /// Returns false if no period is active.
    pub fn verify_temporal_symmetry(&self) -> bool {
        let period = match &self.current_period {
            Some(p) => p,
            None => return false,
        };

        let two_pi = 2.0 * std::f64::consts::PI;

        // Check phase angle bounds
        if period.phase_angle < 0.0 || period.phase_angle >= two_pi {
            return false;
        }

        // Check validators are distinct
        let mut seen = std::collections::HashSet::new();
        for v in &period.validators {
            if !seen.insert(v) {
                return false;
            }
        }

        // Check symmetry group matches
        let expected_group = format!("C{}", period.validators.len());
        if period.symmetry_group != expected_group {
            return false;
        }

        true
    }

    /// Get the current period, if any.
    pub fn current_period(&self) -> Option<&TimeCrystalPeriod> {
        self.current_period.as_ref()
    }

    /// Get the history of completed periods.
    pub fn period_history(&self) -> &[TimeCrystalPeriod] {
        &self.period_history
    }

    /// Total number of completed periods.
    pub fn completed_period_count(&self) -> usize {
        self.period_history.len()
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for TimeCrystalEngine {
    fn primitive_id(&self) -> &str {
        "time_crystal_consensus"
    }

    fn primitive_number(&self) -> u8 {
        20
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Structural
    }

    fn tier(&self) -> u8 {
        3
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Time-crystal consensus is active during Co-Creation when
        // consensus decisions are actually made.
        self.active = new_phase == CyclePhase::CoCreation;
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        if let Some(period) = &self.current_period {
            let two_pi = 2.0 * std::f64::consts::PI;

            // Gate 1: phase_angle in [0, 2*PI)
            let angle_ok = period.phase_angle >= 0.0 && period.phase_angle < two_pi;
            checks.push(Gate1Check {
                invariant: format!("phase_angle in [0, 2*PI) for period {}", period.period_id),
                passed: angle_ok,
                details: if angle_ok {
                    None
                } else {
                    Some(format!("phase_angle = {}", period.phase_angle))
                },
            });

            // Gate 1: period duration must be positive
            let duration_ok = period.period_duration > Duration::zero();
            checks.push(Gate1Check {
                invariant: format!("period_duration positive for period {}", period.period_id),
                passed: duration_ok,
                details: if duration_ok {
                    None
                } else {
                    Some("period_duration is zero or negative".to_string())
                },
            });

            // Gate 1: validators must be non-empty
            let validators_ok = !period.validators.is_empty();
            checks.push(Gate1Check {
                invariant: format!("validators non-empty for period {}", period.period_id),
                passed: validators_ok,
                details: if validators_ok {
                    None
                } else {
                    Some("validator set is empty".to_string())
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        if let Some(period) = &self.current_period {
            // Gate 2: warn if validator set is too small for BFT
            if period.validators.len() < 4 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Sovereignty (Harmony 1)".to_string(),
                    severity: 0.5,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Period {} has only {} validators. BFT consensus typically \
                         requires at least 3f+1 = 4 validators to tolerate 1 fault.",
                        period.period_id,
                        period.validators.len()
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
        let current_phase = self
            .current_period
            .as_ref()
            .map(|p| p.phase_angle)
            .unwrap_or(0.0);
        let current_validators = self
            .current_period
            .as_ref()
            .map(|p| p.validators.len())
            .unwrap_or(0);

        serde_json::json!({
            "primitive": "time_crystal_consensus",
            "primitive_number": 20,
            "has_active_period": self.current_period.is_some(),
            "current_phase_angle": current_phase,
            "current_validators": current_validators,
            "completed_periods": self.period_history.len(),
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

    fn make_engine() -> TimeCrystalEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        TimeCrystalEngine::new(bus)
    }

    fn make_engine_with_bus() -> (TimeCrystalEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = TimeCrystalEngine::new(bus.clone());
        (engine, bus)
    }

    fn test_validators() -> Vec<Did> {
        vec![
            "did:mycelix:v1".to_string(),
            "did:mycelix:v2".to_string(),
            "did:mycelix:v3".to_string(),
            "did:mycelix:v4".to_string(),
        ]
    }

    #[test]
    fn test_start_period() {
        let (mut engine, bus) = make_engine_with_bus();
        let validators = test_validators();

        let event = engine
            .start_period(validators.clone(), Duration::hours(1))
            .unwrap();

        assert_eq!(event.period.period_id, 1);
        assert_eq!(event.period.phase_angle, 0.0);
        assert_eq!(event.period.validators.len(), 4);
        assert_eq!(event.period.symmetry_group, "C4");
        assert!(engine.current_period().is_some());
        assert_eq!(bus.event_count(), 1);
    }

    #[test]
    fn test_cannot_start_period_while_active() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        let result = engine.start_period(test_validators(), Duration::hours(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_start_period_empty_validators() {
        let mut engine = make_engine();
        let result = engine.start_period(vec![], Duration::hours(1));
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_phase_angle() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        let angle = engine.advance_phase_angle(1.0).unwrap();
        assert!((angle - 1.0).abs() < f64::EPSILON);

        let angle2 = engine.advance_phase_angle(0.5).unwrap();
        assert!((angle2 - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_advance_phase_angle_wraps() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        let two_pi = 2.0 * std::f64::consts::PI;

        // Advance past 2*PI
        let angle = engine.advance_phase_angle(two_pi + 1.0).unwrap();
        assert!((angle - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_advance_phase_angle_no_period() {
        let mut engine = make_engine();
        let result = engine.advance_phase_angle(1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_complete_period() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        engine.advance_phase_angle(3.14).unwrap();

        let period = engine.complete_period().unwrap();
        assert_eq!(period.period_id, 1);
        assert!(engine.current_period().is_none());
        assert_eq!(engine.completed_period_count(), 1);
    }

    #[test]
    fn test_complete_period_no_active() {
        let mut engine = make_engine();
        let result = engine.complete_period();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_validator_for_phase() {
        let mut engine = make_engine();
        let validators = test_validators();
        engine
            .start_period(validators.clone(), Duration::hours(1))
            .unwrap();

        let two_pi = 2.0 * std::f64::consts::PI;
        let sector_size = two_pi / 4.0;

        // Phase 0 -> first validator
        let v0 = engine.get_validator_for_phase(0.0).unwrap();
        assert_eq!(v0, validators[0]);

        // Phase in second sector -> second validator
        let v1 = engine.get_validator_for_phase(sector_size + 0.1).unwrap();
        assert_eq!(v1, validators[1]);

        // Phase in third sector -> third validator
        let v2 = engine
            .get_validator_for_phase(2.0 * sector_size + 0.1)
            .unwrap();
        assert_eq!(v2, validators[2]);

        // Phase in fourth sector -> fourth validator
        let v3 = engine
            .get_validator_for_phase(3.0 * sector_size + 0.1)
            .unwrap();
        assert_eq!(v3, validators[3]);
    }

    #[test]
    fn test_get_validator_wraps_negative() {
        let mut engine = make_engine();
        let validators = test_validators();
        engine
            .start_period(validators.clone(), Duration::hours(1))
            .unwrap();

        // Negative phase should wrap
        let v = engine.get_validator_for_phase(-0.1).unwrap();
        // Should map to last validator sector
        assert_eq!(v, validators[3]);
    }

    #[test]
    fn test_get_validator_no_period() {
        let engine = make_engine();
        assert!(engine.get_validator_for_phase(0.0).is_none());
    }

    #[test]
    fn test_verify_temporal_symmetry() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        assert!(engine.verify_temporal_symmetry());
    }

    #[test]
    fn test_verify_temporal_symmetry_no_period() {
        let engine = make_engine();
        assert!(!engine.verify_temporal_symmetry());
    }

    #[test]
    fn test_multiple_periods_sequential() {
        let mut engine = make_engine();

        // Period 1
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();
        engine.advance_phase_angle(3.0).unwrap();
        engine.complete_period().unwrap();

        // Period 2
        engine
            .start_period(test_validators(), Duration::hours(2))
            .unwrap();
        engine.advance_phase_angle(1.0).unwrap();
        engine.complete_period().unwrap();

        assert_eq!(engine.completed_period_count(), 2);
        assert!(engine.current_period().is_none());

        let history = engine.period_history();
        assert_eq!(history[0].period_id, 1);
        assert_eq!(history[1].period_id, 2);
    }

    #[test]
    fn test_gate1_checks_pass_normal() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate2_warns_small_validator_set() {
        let mut engine = make_engine();
        engine
            .start_period(
                vec!["did:mycelix:v1".to_string(), "did:mycelix:v2".to_string()],
                Duration::hours(1),
            )
            .unwrap();

        let warnings = engine.gate2_check();
        assert!(!warnings.is_empty());
        assert!(warnings[0].reasoning.contains("BFT"));
    }

    #[test]
    fn test_gate2_no_warning_sufficient_validators() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();

        let warnings = engine.gate2_check();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "time_crystal_consensus");
        assert_eq!(engine.primitive_number(), 20);
        assert_eq!(engine.module(), PrimitiveModule::Structural);
        assert_eq!(engine.tier(), 3);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::Kenosis));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine
            .start_period(test_validators(), Duration::hours(1))
            .unwrap();
        engine.advance_phase_angle(1.5).unwrap();

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["has_active_period"], true);
        assert_eq!(metrics["current_validators"], 4);
        assert_eq!(metrics["completed_periods"], 0);
        let phase = metrics["current_phase_angle"].as_f64().unwrap();
        assert!((phase - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_deterministic_validator_rotation() {
        // Verifies that advancing through a full cycle visits all validators
        let mut engine = make_engine();
        let validators = test_validators();
        engine
            .start_period(validators.clone(), Duration::hours(1))
            .unwrap();

        let two_pi = 2.0 * std::f64::consts::PI;
        let steps = 100;
        let step_size = two_pi / steps as f64;

        let mut seen_validators = std::collections::HashSet::new();
        for i in 0..steps {
            let angle = i as f64 * step_size;
            if let Some(v) = engine.get_validator_for_phase(angle) {
                seen_validators.insert(v);
            }
        }

        // All validators should have been selected at some phase
        assert_eq!(
            seen_validators.len(),
            validators.len(),
            "All validators should participate across a full cycle"
        );
    }
}
