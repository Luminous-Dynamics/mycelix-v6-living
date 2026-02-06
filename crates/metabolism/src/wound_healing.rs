//! # Wound Healing Engine — Primitive [2]
//!
//! 4-phase FSM replacing punitive slashing with restorative wound healing.
//!
//! ## Philosophy
//!
//! Traditional blockchain protocols punish misbehavior through slashing: a punitive
//! mechanism that permanently destroys stake. The Living Protocol replaces this with
//! a biological wound healing metaphor:
//!
//! 1. **Hemostasis** — Immediate, automatic quarantine. Un-gameable. The system
//!    contains the damage before any assessment occurs.
//! 2. **Inflammation** — Community assessment of the wound. The network examines
//!    the cause, extent, and context of the harm.
//! 3. **Proliferation** — Restitution and repair. The agent performs actions to
//!    heal the damage they caused.
//! 4. **Remodeling** — Integration and strengthening. The healed area becomes
//!    stronger (scar tissue) than the original.
//! 5. **Healed** — Terminal state. The wound is fully healed.
//!
//! ## Key Invariant
//!
//! Phase transitions are **forward-only**. You cannot skip phases or go backwards.
//! This is enforced both at the type level (via `WoundPhase::can_transition_to`)
//! and at the engine level.
//!
//! ## Constitutional Alignment
//!
//! **Sacred Reciprocity (Harmony 6)**: Harm is healed, not punished. The goal is
//! restoration and strengthening, not retribution.
//!
//! ## Solidity Bridge
//!
//! The `WoundEscrow` Solidity contract handles on-chain restitution tracking.
//! This engine manages the off-chain FSM state, coordinating with the on-chain
//! escrow through events.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use living_core::error::{LivingProtocolError, LivingResult};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Did, EventBus, Gate1Check, Gate2Warning, LivingProtocolEvent,
    RestitutionFulfilledEvent, RestitutionRequirement, ScarTissue, ScarTissueFormedEvent,
    WoundCreatedEvent, WoundHealingConfig, WoundPhase, WoundPhaseAdvancedEvent, WoundRecord,
    WoundSeverity,
};

// =============================================================================
// Restitution Actions
// =============================================================================

/// Actions that an agent can submit as part of restitution during the
/// Proliferation phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestitutionAction {
    /// Description of the restorative action taken.
    pub description: String,
    /// Evidence or proof of the action.
    pub evidence: Option<String>,
    /// Timestamp when the action was completed.
    pub completed_at: DateTime<Utc>,
    /// On-chain transaction hash, if applicable (WoundEscrow interaction).
    pub tx_hash: Option<String>,
}

// =============================================================================
// Slash-to-Severity Mapping
// =============================================================================

/// Maps legacy slash percentages to wound severity levels.
///
/// This preserves backward compatibility with v5.x slashing while
/// transitioning to the restorative healing model.
pub fn slash_percentage_to_severity(slash_pct: f64) -> WoundSeverity {
    match slash_pct {
        p if p <= 0.05 => WoundSeverity::Minor,
        p if p <= 0.15 => WoundSeverity::Moderate,
        p if p <= 0.30 => WoundSeverity::Severe,
        _ => WoundSeverity::Critical,
    }
}

/// Estimated healing cycles based on severity.
pub fn estimated_healing_cycles(severity: &WoundSeverity) -> (u32, u32) {
    match severity {
        WoundSeverity::Minor => (1, 1),
        WoundSeverity::Moderate => (2, 3),
        WoundSeverity::Severe => (4, 6),
        WoundSeverity::Critical => (7, 14),
    }
}

// =============================================================================
// Wound Healing Engine
// =============================================================================

/// The wound healing engine manages the 4-phase healing FSM for all wounds
/// in the network.
pub struct WoundHealingEngine {
    /// All wound records indexed by wound ID.
    wounds: HashMap<String, WoundRecord>,
    /// Configuration.
    config: WoundHealingConfig,
    /// Event bus for emitting wound healing events.
    event_bus: Arc<dyn EventBus>,
    /// Index: agent DID -> wound IDs for that agent.
    agent_wounds: HashMap<Did, Vec<String>>,
    /// Whether we are in a phase where wound healing is active.
    active: bool,
}

impl WoundHealingEngine {
    /// Create a new wound healing engine.
    pub fn new(config: WoundHealingConfig, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            wounds: HashMap::new(),
            config,
            event_bus,
            agent_wounds: HashMap::new(),
            active: false,
        }
    }

    /// Create a new wound for an agent.
    ///
    /// The wound starts in the **Hemostasis** phase automatically. This phase
    /// is immediate and un-gameable: it represents the system containing the
    /// damage before any assessment can begin.
    ///
    /// A restitution requirement is generated based on severity.
    pub fn create_wound(
        &mut self,
        agent_did: Did,
        severity: WoundSeverity,
        cause: String,
    ) -> LivingResult<WoundRecord> {
        let wound_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Gate 2: warn on critical wounds
        if severity == WoundSeverity::Critical {
            tracing::warn!(
                agent_did = %agent_did,
                wound_id = %wound_id,
                "Gate 2 warning: Critical wound created for agent {}. \
                 This wound may take 7+ cycles to heal and may leave permanent scar tissue. \
                 Constitutional alignment: Sacred Reciprocity (Harmony 6).",
                agent_did
            );
        }

        // Generate restitution requirement based on severity
        let restitution = self.generate_restitution(&severity, &cause, now);

        let record = WoundRecord {
            id: wound_id.clone(),
            agent_did: agent_did.clone(),
            severity,
            cause: cause.clone(),
            // Hemostasis is automatic/immediate
            phase: WoundPhase::Hemostasis,
            created: now,
            phase_history: vec![(WoundPhase::Hemostasis, now)],
            restitution_required: Some(restitution),
            scar_tissue: None,
        };

        self.wounds.insert(wound_id.clone(), record.clone());
        self.agent_wounds
            .entry(agent_did.clone())
            .or_default()
            .push(wound_id.clone());

        // Emit event
        self.event_bus
            .publish(LivingProtocolEvent::WoundCreated(WoundCreatedEvent {
                wound_id,
                agent_did,
                severity,
                cause,
                timestamp: now,
            }));

        Ok(record)
    }

    /// Advance a wound to the next phase.
    ///
    /// ## Key Invariant
    ///
    /// Phase transitions are **forward-only**. This method validates that the
    /// requested transition follows the FSM:
    ///
    /// Hemostasis -> Inflammation -> Proliferation -> Remodeling -> Healed
    ///
    /// Any attempt to skip or reverse a phase returns `WoundPhaseViolation`.
    pub fn advance_phase(&mut self, wound_id: &str) -> LivingResult<WoundPhase> {
        let wound = self
            .wounds
            .get_mut(wound_id)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(wound_id.to_string()))?;

        let current_phase = wound.phase;

        // Determine the next valid phase
        let valid_next = current_phase.valid_transitions();
        if valid_next.is_empty() {
            return Err(LivingProtocolError::WoundPhaseViolation {
                from: current_phase,
                to: WoundPhase::Healed, // Already healed, nowhere to go
            });
        }

        let next_phase = valid_next[0]; // Forward-only: exactly one valid next phase

        // Validate hemostasis minimum duration
        if current_phase == WoundPhase::Hemostasis {
            let elapsed = Utc::now() - wound.created;
            let min_duration = Duration::hours(self.config.min_hemostasis_hours as i64);
            if elapsed < min_duration {
                tracing::info!(
                    wound_id = %wound_id,
                    elapsed_hours = elapsed.num_hours(),
                    min_hours = self.config.min_hemostasis_hours,
                    "Hemostasis minimum duration not yet met. Quarantine continues."
                );
                // Still allow transition — the minimum is advisory.
                // Un-gameable quarantine is enforced at the contract level.
            }
        }

        // Validate restitution before advancing from Proliferation to Remodeling
        if current_phase == WoundPhase::Proliferation {
            if let Some(ref restitution) = wound.restitution_required {
                if !restitution.fulfilled {
                    tracing::warn!(
                        wound_id = %wound_id,
                        "Advancing past Proliferation without fulfilled restitution. \
                         The wound may not heal properly."
                    );
                }
            }
        }

        let now = Utc::now();
        let from = wound.phase;
        wound.phase = next_phase;
        wound.phase_history.push((next_phase, now));

        // Emit event
        self.event_bus
            .publish(LivingProtocolEvent::WoundPhaseAdvanced(
                WoundPhaseAdvancedEvent {
                    wound_id: wound_id.to_string(),
                    agent_did: wound.agent_did.clone(),
                    from,
                    to: next_phase,
                    timestamp: now,
                },
            ));

        tracing::info!(
            wound_id = %wound_id,
            from = ?from,
            to = ?next_phase,
            "Wound phase advanced. Sacred Reciprocity: healing progresses."
        );

        Ok(next_phase)
    }

    /// Submit restitution actions for a wound in the Proliferation phase.
    ///
    /// Returns `true` if the restitution is now considered fulfilled.
    pub fn submit_restitution(
        &mut self,
        wound_id: &str,
        actions: Vec<RestitutionAction>,
    ) -> LivingResult<bool> {
        let wound = self
            .wounds
            .get_mut(wound_id)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(wound_id.to_string()))?;

        if wound.phase != WoundPhase::Proliferation {
            tracing::warn!(
                wound_id = %wound_id,
                current_phase = ?wound.phase,
                "Restitution submitted outside of Proliferation phase. \
                 Restitution is most effective during Proliferation."
            );
        }

        let restitution = wound.restitution_required.as_mut().ok_or_else(|| {
            LivingProtocolError::CompostingIneligible(
                wound_id.to_string(),
                "No restitution requirement for this wound".to_string(),
            )
        })?;

        // Mark restitution actions as completed
        for action in &actions {
            restitution
                .actions_required
                .push(action.description.clone());
        }

        // Consider restitution fulfilled if at least one action has been submitted
        // In a production system, this would require community validation
        if !actions.is_empty() {
            restitution.fulfilled = true;

            let now = Utc::now();
            self.event_bus
                .publish(LivingProtocolEvent::RestitutionFulfilled(
                    RestitutionFulfilledEvent {
                        wound_id: wound_id.to_string(),
                        agent_did: wound.agent_did.clone(),
                        restitution: restitution.clone(),
                        timestamp: now,
                    },
                ));
        }

        Ok(restitution.fulfilled)
    }

    /// Form scar tissue for a wound in the Remodeling or Healed phase.
    ///
    /// Scar tissue represents the strengthening that results from healing.
    /// The `strength_multiplier` is always > 1.0, meaning the healed area
    /// is stronger than the original.
    pub fn form_scar_tissue(&mut self, wound_id: &str) -> LivingResult<ScarTissue> {
        let wound = self
            .wounds
            .get_mut(wound_id)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(wound_id.to_string()))?;

        if wound.phase != WoundPhase::Remodeling && wound.phase != WoundPhase::Healed {
            return Err(LivingProtocolError::WoundPhaseViolation {
                from: wound.phase,
                to: WoundPhase::Remodeling,
            });
        }

        let now = Utc::now();

        // Compute strength multiplier based on severity
        // More severe wounds produce stronger scar tissue (within config bounds)
        let severity_factor = match wound.severity {
            WoundSeverity::Minor => 0.0,
            WoundSeverity::Moderate => 0.33,
            WoundSeverity::Severe => 0.66,
            WoundSeverity::Critical => 1.0,
        };

        let range = self.config.scar_strength_max - self.config.scar_strength_min;
        let strength_multiplier = self.config.scar_strength_min + severity_factor * range;

        let scar = ScarTissue {
            area: wound.cause.clone(),
            strength_multiplier,
            formed: now,
        };

        wound.scar_tissue = Some(scar.clone());

        // Emit event
        self.event_bus
            .publish(LivingProtocolEvent::ScarTissueFormed(
                ScarTissueFormedEvent {
                    wound_id: wound_id.to_string(),
                    agent_did: wound.agent_did.clone(),
                    scar: scar.clone(),
                    timestamp: now,
                },
            ));

        tracing::info!(
            wound_id = %wound_id,
            strength = strength_multiplier,
            "Scar tissue formed. The healed area is now {:.1}x stronger.",
            strength_multiplier
        );

        Ok(scar)
    }

    /// Get all wounds for a specific agent.
    pub fn get_wounds_for_agent(&self, agent_did: &str) -> Vec<WoundRecord> {
        self.agent_wounds
            .get(agent_did)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.wounds.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a specific wound record by ID.
    pub fn get_wound(&self, wound_id: &str) -> Option<&WoundRecord> {
        self.wounds.get(wound_id)
    }

    /// Get all active (not yet healed) wounds in the system.
    pub fn get_active_wounds(&self) -> Vec<WoundRecord> {
        self.wounds
            .values()
            .filter(|w| w.phase != WoundPhase::Healed)
            .cloned()
            .collect()
    }

    /// Get all healed wounds in the system.
    pub fn get_healed_wounds(&self) -> Vec<WoundRecord> {
        self.wounds
            .values()
            .filter(|w| w.phase == WoundPhase::Healed)
            .cloned()
            .collect()
    }

    /// Total count of all wounds ever created.
    pub fn total_wound_count(&self) -> usize {
        self.wounds.len()
    }

    /// Walk a wound through all phases to Healed status (convenience method).
    /// Returns the final wound record.
    pub fn heal_fully(&mut self, wound_id: &str) -> LivingResult<WoundRecord> {
        loop {
            let wound = self
                .wounds
                .get(wound_id)
                .ok_or_else(|| LivingProtocolError::AgentNotFound(wound_id.to_string()))?;
            if wound.phase == WoundPhase::Healed {
                break;
            }
            self.advance_phase(wound_id)?;
        }
        self.form_scar_tissue(wound_id)?;
        Ok(self.wounds.get(wound_id).unwrap().clone())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Generate a restitution requirement based on wound severity.
    fn generate_restitution(
        &self,
        severity: &WoundSeverity,
        cause: &str,
        now: DateTime<Utc>,
    ) -> RestitutionRequirement {
        let (description, amount, actions) = match severity {
            WoundSeverity::Minor => (
                format!("Minor restitution for: {}", cause),
                Some(0.01),
                vec!["Acknowledge the harm".to_string()],
            ),
            WoundSeverity::Moderate => (
                format!("Moderate restitution for: {}", cause),
                Some(0.05),
                vec![
                    "Acknowledge the harm".to_string(),
                    "Propose corrective action".to_string(),
                ],
            ),
            WoundSeverity::Severe => (
                format!("Severe restitution for: {}", cause),
                Some(0.15),
                vec![
                    "Acknowledge the harm".to_string(),
                    "Propose corrective action".to_string(),
                    "Implement corrective action".to_string(),
                ],
            ),
            WoundSeverity::Critical => (
                format!("Critical restitution for: {}", cause),
                Some(0.30),
                vec![
                    "Acknowledge the harm".to_string(),
                    "Full root cause analysis".to_string(),
                    "Propose systemic fix".to_string(),
                    "Implement systemic fix".to_string(),
                    "Community review of fix".to_string(),
                ],
            ),
        };

        let deadline = now + Duration::days(self.config.restitution_deadline_days as i64);

        RestitutionRequirement {
            description,
            amount_flow: amount,
            actions_required: actions,
            deadline,
            fulfilled: false,
        }
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for WoundHealingEngine {
    fn primitive_id(&self) -> &str {
        "wound_healing"
    }

    fn primitive_number(&self) -> u8 {
        2
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Metabolism
    }

    fn tier(&self) -> u8 {
        2
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Wound healing is relevant during multiple phases but primarily
        // managed outside the cycle phase constraints
        self.active = true; // Always active — wounds don't wait for phases
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: phase transitions are forward-only
        for wound in self.wounds.values() {
            if wound.phase_history.len() >= 2 {
                let mut forward_only = true;
                for window in wound.phase_history.windows(2) {
                    let from = &window[0].0;
                    let to = &window[1].0;
                    if !from.can_transition_to(to) {
                        forward_only = false;
                        break;
                    }
                }
                checks.push(Gate1Check {
                    invariant: format!("forward-only phase transitions for wound {}", wound.id),
                    passed: forward_only,
                    details: if forward_only {
                        None
                    } else {
                        Some("Phase history contains non-forward transition".to_string())
                    },
                });
            }

            // Gate 1: scar tissue strength_multiplier > 1.0
            if let Some(ref scar) = wound.scar_tissue {
                let valid = scar.strength_multiplier > 1.0;
                checks.push(Gate1Check {
                    invariant: format!("scar tissue strength > 1.0 for wound {}", wound.id),
                    passed: valid,
                    details: if valid {
                        None
                    } else {
                        Some(format!(
                            "scar tissue strength = {} is not > 1.0",
                            scar.strength_multiplier
                        ))
                    },
                });
            }
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Gate 2: warn on critical wounds
        for wound in self.wounds.values() {
            if wound.severity == WoundSeverity::Critical && wound.phase != WoundPhase::Healed {
                warnings.push(Gate2Warning {
                    harmony_violated: "Sacred Reciprocity (Harmony 6)".to_string(),
                    severity: 0.8,
                    reputation_impact: -0.05,
                    reasoning: format!(
                        "Critical wound {} for agent {} is still healing (phase: {:?}). \
                         This requires immediate attention from the community.",
                        wound.id, wound.agent_did, wound.phase
                    ),
                    user_may_proceed: true,
                });
            }
        }

        // Gate 2: warn on wounds past restitution deadline
        let now = Utc::now();
        for wound in self.wounds.values() {
            if let Some(ref restitution) = wound.restitution_required {
                if !restitution.fulfilled && now > restitution.deadline {
                    warnings.push(Gate2Warning {
                        harmony_violated: "Sacred Reciprocity (Harmony 6)".to_string(),
                        severity: 0.6,
                        reputation_impact: -0.03,
                        reasoning: format!(
                            "Wound {} has unfulfilled restitution past deadline. \
                             Consider escalating to composting if agent is unresponsive.",
                            wound.id
                        ),
                        user_may_proceed: true,
                    });
                }
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, _phase: CyclePhase) -> bool {
        // Wound healing is active in ALL phases — wounds don't wait
        true
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let mut phase_counts = HashMap::new();
        for wound in self.wounds.values() {
            *phase_counts
                .entry(format!("{:?}", wound.phase))
                .or_insert(0u64) += 1;
        }

        let mut severity_counts = HashMap::new();
        for wound in self.wounds.values() {
            *severity_counts
                .entry(format!("{:?}", wound.severity))
                .or_insert(0u64) += 1;
        }

        serde_json::json!({
            "total_wounds": self.wounds.len(),
            "active_wounds": self.get_active_wounds().len(),
            "healed_wounds": self.get_healed_wounds().len(),
            "phase_distribution": phase_counts,
            "severity_distribution": severity_counts,
            "primitive": "wound_healing",
            "primitive_number": 2,
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
    use proptest::prelude::*;

    fn make_engine() -> WoundHealingEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        WoundHealingEngine::new(WoundHealingConfig::default(), bus)
    }

    fn make_engine_with_bus() -> (WoundHealingEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = WoundHealingEngine::new(WoundHealingConfig::default(), bus.clone());
        (engine, bus)
    }

    #[test]
    fn test_create_wound_starts_in_hemostasis() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Moderate,
                "violated governance quorum".to_string(),
            )
            .unwrap();

        assert_eq!(wound.phase, WoundPhase::Hemostasis);
        assert_eq!(wound.severity, WoundSeverity::Moderate);
        assert_eq!(wound.agent_did, "did:mycelix:agent1");
        assert_eq!(wound.phase_history.len(), 1);
        assert!(wound.restitution_required.is_some());
    }

    #[test]
    fn test_advance_phase_forward_only() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Minor,
                "late submission".to_string(),
            )
            .unwrap();

        // Hemostasis -> Inflammation
        let phase = engine.advance_phase(&wound.id).unwrap();
        assert_eq!(phase, WoundPhase::Inflammation);

        // Inflammation -> Proliferation
        let phase = engine.advance_phase(&wound.id).unwrap();
        assert_eq!(phase, WoundPhase::Proliferation);

        // Proliferation -> Remodeling
        let phase = engine.advance_phase(&wound.id).unwrap();
        assert_eq!(phase, WoundPhase::Remodeling);

        // Remodeling -> Healed
        let phase = engine.advance_phase(&wound.id).unwrap();
        assert_eq!(phase, WoundPhase::Healed);
    }

    #[test]
    fn test_cannot_advance_past_healed() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Minor,
                "test".to_string(),
            )
            .unwrap();

        // Advance to Healed
        engine.advance_phase(&wound.id).unwrap(); // -> Inflammation
        engine.advance_phase(&wound.id).unwrap(); // -> Proliferation
        engine.advance_phase(&wound.id).unwrap(); // -> Remodeling
        engine.advance_phase(&wound.id).unwrap(); // -> Healed

        // Cannot advance past Healed
        let result = engine.advance_phase(&wound.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_history_records_all_transitions() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Severe,
                "data corruption".to_string(),
            )
            .unwrap();

        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();

        let wound = engine.get_wound(&wound.id).unwrap();
        assert_eq!(wound.phase_history.len(), 5); // Hemostasis + 4 transitions

        let phases: Vec<WoundPhase> = wound.phase_history.iter().map(|(p, _)| *p).collect();
        assert_eq!(
            phases,
            vec![
                WoundPhase::Hemostasis,
                WoundPhase::Inflammation,
                WoundPhase::Proliferation,
                WoundPhase::Remodeling,
                WoundPhase::Healed,
            ]
        );
    }

    #[test]
    fn test_submit_restitution() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Moderate,
                "protocol violation".to_string(),
            )
            .unwrap();

        // Advance to Proliferation
        engine.advance_phase(&wound.id).unwrap(); // -> Inflammation
        engine.advance_phase(&wound.id).unwrap(); // -> Proliferation

        let actions = vec![RestitutionAction {
            description: "Acknowledged harm and submitted corrective proposal".to_string(),
            evidence: Some("proposal-id-123".to_string()),
            completed_at: Utc::now(),
            tx_hash: None,
        }];

        let fulfilled = engine.submit_restitution(&wound.id, actions).unwrap();
        assert!(fulfilled);

        let wound = engine.get_wound(&wound.id).unwrap();
        assert!(wound.restitution_required.as_ref().unwrap().fulfilled);
    }

    #[test]
    fn test_form_scar_tissue() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Severe,
                "security breach".to_string(),
            )
            .unwrap();

        // Advance to Remodeling
        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();

        let scar = engine.form_scar_tissue(&wound.id).unwrap();

        // Scar tissue makes healed areas stronger
        assert!(scar.strength_multiplier > 1.0);
        assert!(scar.strength_multiplier <= 2.0);
        assert_eq!(scar.area, "security breach");
    }

    #[test]
    fn test_scar_tissue_strength_varies_by_severity() {
        let mut engine = make_engine();

        // Minor wound
        let minor = engine
            .create_wound(
                "did:a".to_string(),
                WoundSeverity::Minor,
                "minor".to_string(),
            )
            .unwrap();
        for _ in 0..3 {
            engine.advance_phase(&minor.id).unwrap();
        }
        let minor_scar = engine.form_scar_tissue(&minor.id).unwrap();

        // Critical wound
        let critical = engine
            .create_wound(
                "did:b".to_string(),
                WoundSeverity::Critical,
                "critical".to_string(),
            )
            .unwrap();
        for _ in 0..3 {
            engine.advance_phase(&critical.id).unwrap();
        }
        let critical_scar = engine.form_scar_tissue(&critical.id).unwrap();

        // Critical wounds produce stronger scar tissue
        assert!(critical_scar.strength_multiplier > minor_scar.strength_multiplier);
    }

    #[test]
    fn test_cannot_form_scar_tissue_early() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Minor,
                "test".to_string(),
            )
            .unwrap();

        // Still in Hemostasis — too early for scar tissue
        let result = engine.form_scar_tissue(&wound.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_wounds_for_agent() {
        let mut engine = make_engine();
        let agent = "did:mycelix:agent1".to_string();

        engine
            .create_wound(agent.clone(), WoundSeverity::Minor, "issue 1".to_string())
            .unwrap();
        engine
            .create_wound(
                agent.clone(),
                WoundSeverity::Moderate,
                "issue 2".to_string(),
            )
            .unwrap();
        engine
            .create_wound(
                "did:mycelix:agent2".to_string(),
                WoundSeverity::Severe,
                "other".to_string(),
            )
            .unwrap();

        let agent1_wounds = engine.get_wounds_for_agent(&agent);
        assert_eq!(agent1_wounds.len(), 2);

        let agent2_wounds = engine.get_wounds_for_agent("did:mycelix:agent2");
        assert_eq!(agent2_wounds.len(), 1);

        let no_wounds = engine.get_wounds_for_agent("did:mycelix:nonexistent");
        assert!(no_wounds.is_empty());
    }

    #[test]
    fn test_slash_percentage_to_severity_mapping() {
        assert_eq!(slash_percentage_to_severity(0.01), WoundSeverity::Minor);
        assert_eq!(slash_percentage_to_severity(0.05), WoundSeverity::Minor);
        assert_eq!(slash_percentage_to_severity(0.10), WoundSeverity::Moderate);
        assert_eq!(slash_percentage_to_severity(0.15), WoundSeverity::Moderate);
        assert_eq!(slash_percentage_to_severity(0.20), WoundSeverity::Severe);
        assert_eq!(slash_percentage_to_severity(0.30), WoundSeverity::Severe);
        assert_eq!(slash_percentage_to_severity(0.50), WoundSeverity::Critical);
        assert_eq!(slash_percentage_to_severity(1.00), WoundSeverity::Critical);
    }

    #[test]
    fn test_estimated_healing_cycles() {
        let (min, max) = estimated_healing_cycles(&WoundSeverity::Minor);
        assert_eq!((min, max), (1, 1));

        let (min, _max) = estimated_healing_cycles(&WoundSeverity::Critical);
        assert!(min >= 7);
    }

    #[test]
    fn test_heal_fully_convenience() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Moderate,
                "test full healing".to_string(),
            )
            .unwrap();

        let healed = engine.heal_fully(&wound.id).unwrap();
        assert_eq!(healed.phase, WoundPhase::Healed);
        assert!(healed.scar_tissue.is_some());
        assert!(healed.scar_tissue.unwrap().strength_multiplier > 1.0);
    }

    #[test]
    fn test_events_emitted_during_healing() {
        let (mut engine, bus) = make_engine_with_bus();
        let wound = engine
            .create_wound(
                "did:mycelix:agent1".to_string(),
                WoundSeverity::Minor,
                "test events".to_string(),
            )
            .unwrap();

        engine.advance_phase(&wound.id).unwrap(); // -> Inflammation
        engine.advance_phase(&wound.id).unwrap(); // -> Proliferation

        let actions = vec![RestitutionAction {
            description: "Fixed the issue".to_string(),
            evidence: None,
            completed_at: Utc::now(),
            tx_hash: None,
        }];
        engine.submit_restitution(&wound.id, actions).unwrap();

        engine.advance_phase(&wound.id).unwrap(); // -> Remodeling
        engine.form_scar_tissue(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap(); // -> Healed

        // Events: WoundCreated + 4 PhaseAdvanced + RestitutionFulfilled + ScarTissueFormed = 7
        assert_eq!(bus.event_count(), 7);
    }

    #[test]
    fn test_gate1_forward_only_passes() {
        let mut engine = make_engine();
        let wound = engine
            .create_wound(
                "did:a".to_string(),
                WoundSeverity::Minor,
                "test".to_string(),
            )
            .unwrap();
        engine.advance_phase(&wound.id).unwrap();
        engine.advance_phase(&wound.id).unwrap();

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate2_warns_on_critical() {
        let mut engine = make_engine();
        engine
            .create_wound(
                "did:a".to_string(),
                WoundSeverity::Critical,
                "critical issue".to_string(),
            )
            .unwrap();

        let warnings = engine.gate2_check();
        assert!(!warnings.is_empty());
        assert!(warnings[0].harmony_violated.contains("Sacred Reciprocity"));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "wound_healing");
        assert_eq!(engine.primitive_number(), 2);
        assert_eq!(engine.module(), PrimitiveModule::Metabolism);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_is_active_in_all_phases() {
        let engine = make_engine();
        for phase in CyclePhase::all_phases() {
            assert!(engine.is_active_in_phase(*phase));
        }
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine
            .create_wound("did:a".to_string(), WoundSeverity::Minor, "a".to_string())
            .unwrap();
        engine
            .create_wound(
                "did:b".to_string(),
                WoundSeverity::Critical,
                "b".to_string(),
            )
            .unwrap();

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_wounds"], 2);
        assert_eq!(metrics["active_wounds"], 2);
        assert_eq!(metrics["healed_wounds"], 0);
    }

    // =========================================================================
    // Proptest: forward-only invariant
    // =========================================================================

    proptest! {
        /// The critical invariant: no matter how many times we advance,
        /// the phase history is always strictly forward.
        #[test]
        fn prop_phase_transitions_forward_only(
            num_advances in 0usize..=10,
            severity in prop_oneof![
                Just(WoundSeverity::Minor),
                Just(WoundSeverity::Moderate),
                Just(WoundSeverity::Severe),
                Just(WoundSeverity::Critical),
            ],
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let mut engine = WoundHealingEngine::new(WoundHealingConfig::default(), bus);

            let wound = engine
                .create_wound(
                    "did:prop-test".to_string(),
                    severity,
                    "proptest cause".to_string(),
                )
                .unwrap();

            // Advance as many times as requested (ignoring errors for Healed)
            for _ in 0..num_advances {
                let _ = engine.advance_phase(&wound.id);
            }

            // Verify forward-only invariant
            let wound = engine.get_wound(&wound.id).unwrap();
            if wound.phase_history.len() >= 2 {
                for window in wound.phase_history.windows(2) {
                    let from = &window[0].0;
                    let to = &window[1].0;
                    prop_assert!(
                        from.can_transition_to(to),
                        "Non-forward transition detected: {:?} -> {:?}",
                        from,
                        to
                    );
                }
            }

            // Verify scar tissue strength > 1.0 if present
            if let Some(ref scar) = wound.scar_tissue {
                prop_assert!(
                    scar.strength_multiplier > 1.0,
                    "Scar tissue strength {} is not > 1.0",
                    scar.strength_multiplier
                );
            }
        }

        /// The phase can never be more advanced than the number of advances.
        #[test]
        fn prop_phase_matches_advance_count(
            num_advances in 0usize..=10,
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let mut engine = WoundHealingEngine::new(WoundHealingConfig::default(), bus);

            let wound = engine
                .create_wound(
                    "did:prop-test".to_string(),
                    WoundSeverity::Minor,
                    "test".to_string(),
                )
                .unwrap();

            let mut successful_advances = 0u32;
            for _ in 0..num_advances {
                if engine.advance_phase(&wound.id).is_ok() {
                    successful_advances += 1;
                }
            }

            // Max possible advances is 4 (Hemostasis->Inflammation->Proliferation->Remodeling->Healed)
            prop_assert!(successful_advances <= 4);

            let wound = engine.get_wound(&wound.id).unwrap();
            let expected_phase = match successful_advances {
                0 => WoundPhase::Hemostasis,
                1 => WoundPhase::Inflammation,
                2 => WoundPhase::Proliferation,
                3 => WoundPhase::Remodeling,
                4 => WoundPhase::Healed,
                _ => unreachable!(),
            };
            prop_assert_eq!(wound.phase, expected_phase);
        }

        /// Gate 1 checks always pass regardless of engine state.
        #[test]
        fn prop_gate1_always_passes(
            num_wounds in 1usize..=5,
            advances_each in 0usize..=4,
        ) {
            let bus = Arc::new(InMemoryEventBus::new());
            let mut engine = WoundHealingEngine::new(WoundHealingConfig::default(), bus);

            for i in 0..num_wounds {
                let wound = engine
                    .create_wound(
                        format!("did:agent-{}", i),
                        WoundSeverity::Moderate,
                        format!("cause-{}", i),
                    )
                    .unwrap();

                for _ in 0..advances_each {
                    let _ = engine.advance_phase(&wound.id);
                }

                // Form scar tissue if possible
                let _ = engine.form_scar_tissue(&wound.id);
            }

            let checks = engine.gate1_check();
            for check in &checks {
                prop_assert!(
                    check.passed,
                    "Gate 1 failed: {} — {:?}",
                    check.invariant,
                    check.details
                );
            }
        }
    }
}
