//! # Collective Dreaming Engine [Primitive 7]
//!
//! Circadian state machine governing the network's creative unconscious.
//! The network cycles through four states:
//!
//! - **Waking**: Normal operation. Dream proposals can be confirmed here.
//! - **REM**: Pattern recombination, creative exploration.
//! - **Deep**: Memory consolidation, structural optimization.
//! - **Lucid**: Conscious dreaming, guided exploration.
//!
//! ## Safeguards
//!
//! 1. Dream outputs are **NON-BINDING** until confirmed during Waking state
//!    with a **0.67 supermajority** threshold.
//! 2. **NO financial transactions** are permitted during any dream state
//!    (REM, Deep, or Lucid).
//! 3. **Gate 1 invariants** are enforced at all times, including during dreams.
//! 4. Valid transitions: Waking <-> REM <-> Deep <-> Lucid (no skipping states).
//!
//! ## Tier
//! Tier 3 (experimental). Requires `tier3-experimental` feature flag.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use living_core::{
    DreamState, DreamProposal,
    DreamStateChangedEvent, DreamProposalGeneratedEvent,
    LivingProtocolEvent,
    Gate1Check, Gate2Warning,
    CyclePhase,
    LivingProtocolError, LivingResult,
};
use living_core::traits::{LivingPrimitive, PrimitiveModule};

/// Confirmation threshold: 67% supermajority required.
const CONFIRMATION_THRESHOLD: f64 = 0.67;

/// Maximum pending proposals before new submissions are rejected.
const MAX_PENDING_PROPOSALS: usize = 100;

// =============================================================================
// State Transition Validation
// =============================================================================

/// Check whether a state transition is valid.
/// Valid transitions form a linear chain: Waking <-> REM <-> Deep <-> Lucid.
fn is_valid_transition(from: DreamState, to: DreamState) -> bool {
    matches!(
        (from, to),
        (DreamState::Waking, DreamState::Rem)
            | (DreamState::Rem, DreamState::Waking)
            | (DreamState::Rem, DreamState::Deep)
            | (DreamState::Deep, DreamState::Rem)
            | (DreamState::Deep, DreamState::Lucid)
            | (DreamState::Lucid, DreamState::Deep)
    )
}

/// Whether the given state is a dreaming state (not Waking).
fn is_dreaming(state: DreamState) -> bool {
    !matches!(state, DreamState::Waking)
}

// =============================================================================
// CollectiveDreamingEngine
// =============================================================================

/// Engine managing the collective dreaming state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveDreamingEngine {
    /// Current dream state.
    current_state: DreamState,
    /// When the current state was entered.
    state_entered_at: DateTime<Utc>,
    /// History of state transitions.
    transition_history: Vec<(DreamState, DreamState, DateTime<Utc>)>,
    /// Pending dream proposals awaiting confirmation.
    pending_proposals: HashMap<String, DreamProposal>,
    /// Confirmed proposals (passed threshold during Waking).
    confirmed_proposals: Vec<DreamProposal>,
    /// Rejected proposals (failed threshold during Waking).
    rejected_proposals: Vec<DreamProposal>,
    /// Network participation ratio [0.0, 1.0].
    network_participation: f64,
    /// Events emitted since last drain.
    #[serde(skip)]
    pending_events: Vec<LivingProtocolEvent>,
}

impl CollectiveDreamingEngine {
    /// Create a new engine in the Waking state.
    pub fn new() -> Self {
        Self {
            current_state: DreamState::Waking,
            state_entered_at: Utc::now(),
            transition_history: Vec::new(),
            pending_proposals: HashMap::new(),
            confirmed_proposals: Vec::new(),
            rejected_proposals: Vec::new(),
            network_participation: 1.0,
            pending_events: Vec::new(),
        }
    }

    /// Get the current dream state.
    pub fn current_state(&self) -> DreamState {
        self.current_state
    }

    /// Get when the current state was entered.
    pub fn state_entered_at(&self) -> DateTime<Utc> {
        self.state_entered_at
    }

    /// Whether the network is currently in a dreaming state.
    pub fn is_dreaming(&self) -> bool {
        is_dreaming(self.current_state)
    }

    /// Transition to a new dream state.
    ///
    /// ## Errors
    /// - Returns `DreamPhaseRestriction` if the transition is invalid.
    pub fn transition_to(
        &mut self,
        new_state: DreamState,
    ) -> LivingResult<DreamStateChangedEvent> {
        let from = self.current_state;

        if from == new_state {
            return Err(LivingProtocolError::DreamPhaseRestriction(format!(
                "Already in {:?} state",
                new_state
            )));
        }

        if !is_valid_transition(from, new_state) {
            return Err(LivingProtocolError::DreamPhaseRestriction(format!(
                "Invalid transition from {:?} to {:?}. \
                 Valid chain: Waking <-> REM <-> Deep <-> Lucid",
                from, new_state
            )));
        }

        let now = Utc::now();
        self.transition_history.push((from, new_state, now));
        self.current_state = new_state;
        self.state_entered_at = now;

        let event = DreamStateChangedEvent {
            from,
            to: new_state,
            network_participation: self.network_participation,
            timestamp: now,
        };

        info!(
            from = ?from,
            to = ?new_state,
            participation = self.network_participation,
            "Dream state transition"
        );

        self.pending_events
            .push(LivingProtocolEvent::DreamStateChanged(event.clone()));

        Ok(event)
    }

    /// Submit a dream proposal. Can only be submitted during a dream state.
    ///
    /// ## Safeguard
    /// The proposal is created with `confirmed: false` and `financial_operations: false`.
    /// It must be confirmed during the Waking state with 0.67 threshold.
    pub fn submit_dream_proposal(
        &mut self,
        content: String,
    ) -> LivingResult<DreamProposal> {
        if !self.is_dreaming() {
            return Err(LivingProtocolError::DreamPhaseRestriction(
                "Dream proposals can only be submitted during a dream state (REM/Deep/Lucid)"
                    .to_string(),
            ));
        }

        if self.pending_proposals.len() >= MAX_PENDING_PROPOSALS {
            return Err(LivingProtocolError::DreamPhaseRestriction(format!(
                "Maximum pending proposals ({}) reached",
                MAX_PENDING_PROPOSALS
            )));
        }

        let proposal = DreamProposal {
            id: Uuid::new_v4().to_string(),
            dream_state: self.current_state,
            content,
            generated_at: Utc::now(),
            confirmed: false,
            confirmation_threshold: CONFIRMATION_THRESHOLD,
            // SAFEGUARD: No financial operations in dream proposals
            financial_operations: false,
        };

        self.pending_proposals
            .insert(proposal.id.clone(), proposal.clone());

        let event = DreamProposalGeneratedEvent {
            proposal: proposal.clone(),
            timestamp: Utc::now(),
        };

        info!(
            proposal_id = %proposal.id,
            dream_state = ?self.current_state,
            "Dream proposal submitted"
        );

        self.pending_events
            .push(LivingProtocolEvent::DreamProposalGenerated(event));

        Ok(proposal)
    }

    /// Confirm or reject a dream proposal based on vote ratio.
    ///
    /// ## Safeguards
    /// - Must be in Waking state to confirm.
    /// - Requires 0.67 (67%) supermajority to pass.
    /// - Returns `true` if confirmed, `false` if rejected.
    ///
    /// ## Parameters
    /// - `proposal_id`: The ID of the proposal to confirm.
    /// - `votes_for`: Number of votes in favor.
    /// - `votes_total`: Total number of votes cast.
    pub fn confirm_dream_proposal(
        &mut self,
        proposal_id: &str,
        votes_for: u64,
        votes_total: u64,
    ) -> LivingResult<bool> {
        // SAFEGUARD: Only during Waking
        if self.is_dreaming() {
            return Err(LivingProtocolError::DreamPhaseRestriction(
                "Dream proposals can only be confirmed during Waking state".to_string(),
            ));
        }

        let proposal = self
            .pending_proposals
            .remove(proposal_id)
            .ok_or_else(|| {
                LivingProtocolError::DreamPhaseRestriction(format!(
                    "Proposal {} not found in pending proposals",
                    proposal_id
                ))
            })?;

        if votes_total == 0 {
            self.rejected_proposals.push(proposal);
            return Ok(false);
        }

        let vote_ratio = votes_for as f64 / votes_total as f64;
        let confirmed = vote_ratio >= CONFIRMATION_THRESHOLD;

        let mut finalized = proposal;
        finalized.confirmed = confirmed;

        if confirmed {
            info!(
                proposal_id = %finalized.id,
                vote_ratio = vote_ratio,
                "Dream proposal CONFIRMED"
            );
            self.confirmed_proposals.push(finalized);
        } else {
            info!(
                proposal_id = %finalized.id,
                vote_ratio = vote_ratio,
                threshold = CONFIRMATION_THRESHOLD,
                "Dream proposal REJECTED (below threshold)"
            );
            self.rejected_proposals.push(finalized);
        }

        Ok(confirmed)
    }

    /// Whether financial transactions are blocked.
    ///
    /// ## SAFEGUARD
    /// Returns `true` during ANY dream state (REM, Deep, Lucid).
    /// Returns `false` only during Waking.
    pub fn is_financial_blocked(&self) -> bool {
        self.is_dreaming()
    }

    /// Set the network participation ratio.
    pub fn set_network_participation(&mut self, participation: f64) {
        self.network_participation = participation.clamp(0.0, 1.0);
    }

    /// Get the number of pending (unconfirmed) proposals.
    pub fn pending_proposal_count(&self) -> usize {
        self.pending_proposals.len()
    }

    /// Get the number of confirmed proposals.
    pub fn confirmed_proposal_count(&self) -> usize {
        self.confirmed_proposals.len()
    }

    /// Get the number of rejected proposals.
    pub fn rejected_proposal_count(&self) -> usize {
        self.rejected_proposals.len()
    }

    /// Get the transition history.
    pub fn transition_history(&self) -> &[(DreamState, DreamState, DateTime<Utc>)] {
        &self.transition_history
    }

    /// Get a reference to a pending proposal by ID.
    pub fn get_pending_proposal(&self, id: &str) -> Option<&DreamProposal> {
        self.pending_proposals.get(id)
    }

    /// Get all confirmed proposals.
    pub fn confirmed_proposals(&self) -> &[DreamProposal] {
        &self.confirmed_proposals
    }

    /// Drain pending events.
    pub fn drain_events(&mut self) -> Vec<LivingProtocolEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Attempt a financial operation. Always fails during dream states.
    ///
    /// This is an explicit safeguard method that should be called before
    /// any financial operation in the network.
    pub fn guard_financial_operation(&self, operation: &str) -> LivingResult<()> {
        if self.is_financial_blocked() {
            warn!(
                operation = operation,
                state = ?self.current_state,
                "Financial operation BLOCKED during dream state"
            );
            Err(LivingProtocolError::DreamPhaseRestriction(format!(
                "Financial operation '{}' is blocked during {:?} state. \
                 NO financial transactions are permitted during dreaming.",
                operation, self.current_state
            )))
        } else {
            Ok(())
        }
    }
}

impl Default for CollectiveDreamingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LivingPrimitive for CollectiveDreamingEngine {
    fn primitive_id(&self) -> &str {
        "collective_dreaming"
    }

    fn primitive_number(&self) -> u8 {
        7
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Consciousness
    }

    fn tier(&self) -> u8 {
        3
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // When the metabolism cycle transitions, we stay in whatever dream
        // state we are in. The dreaming engine has its own state machine
        // that is independent of the metabolism cycle.
        Ok(self.drain_events())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: All pending proposals must have financial_operations = false
        let all_non_financial = self
            .pending_proposals
            .values()
            .all(|p| !p.financial_operations);

        checks.push(Gate1Check {
            invariant: "dream_no_financial_operations".to_string(),
            passed: all_non_financial,
            details: if all_non_financial {
                None
            } else {
                Some("CRITICAL: A dream proposal has financial_operations=true. This violates Gate 1.".to_string())
            },
        });

        // Gate 1: All pending proposals must have confirmed = false
        let all_unconfirmed = self
            .pending_proposals
            .values()
            .all(|p| !p.confirmed);

        checks.push(Gate1Check {
            invariant: "dream_pending_unconfirmed".to_string(),
            passed: all_unconfirmed,
            details: if all_unconfirmed {
                None
            } else {
                Some("CRITICAL: A pending dream proposal is marked as confirmed.".to_string())
            },
        });

        // Gate 1: Confirmation threshold must be >= 0.67
        let threshold_valid = self
            .pending_proposals
            .values()
            .all(|p| p.confirmation_threshold >= CONFIRMATION_THRESHOLD);

        checks.push(Gate1Check {
            invariant: "dream_confirmation_threshold".to_string(),
            passed: threshold_valid,
            details: if threshold_valid {
                None
            } else {
                Some(format!(
                    "CRITICAL: A proposal has confirmation threshold below {}",
                    CONFIRMATION_THRESHOLD
                ))
            },
        });

        // Gate 1: If dreaming, financial must be blocked
        if self.is_dreaming() {
            checks.push(Gate1Check {
                invariant: "dream_financial_blocked_during_dreaming".to_string(),
                passed: self.is_financial_blocked(),
                details: if self.is_financial_blocked() {
                    None
                } else {
                    Some("CRITICAL: Financial operations not blocked during dream state.".to_string())
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Warn if too many pending proposals
        if self.pending_proposals.len() > 50 {
            warnings.push(Gate2Warning {
                harmony_violated: "Sacred Reciprocity".to_string(),
                severity: 0.5,
                reputation_impact: 0.0,
                reasoning: format!(
                    "Large number of pending dream proposals: {}. Consider reviewing.",
                    self.pending_proposals.len()
                ),
                user_may_proceed: true,
            });
        }

        warnings
    }

    fn is_active_in_phase(&self, _phase: CyclePhase) -> bool {
        // Dreaming can happen in any phase, though it is more natural
        // during certain phases. The engine manages its own state.
        true
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "primitive": "collective_dreaming",
            "current_state": format!("{:?}", self.current_state),
            "pending_proposals": self.pending_proposals.len(),
            "confirmed_proposals": self.confirmed_proposals.len(),
            "rejected_proposals": self.rejected_proposals.len(),
            "transitions": self.transition_history.len(),
            "financial_blocked": self.is_financial_blocked(),
            "network_participation": self.network_participation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // State Machine Tests
    // =========================================================================

    #[test]
    fn test_initial_state_is_waking() {
        let engine = CollectiveDreamingEngine::new();
        assert_eq!(engine.current_state(), DreamState::Waking);
        assert!(!engine.is_dreaming());
        assert!(!engine.is_financial_blocked());
    }

    #[test]
    fn test_valid_transitions_waking_to_rem() {
        let mut engine = CollectiveDreamingEngine::new();
        let event = engine.transition_to(DreamState::Rem).unwrap();
        assert_eq!(event.from, DreamState::Waking);
        assert_eq!(event.to, DreamState::Rem);
        assert_eq!(engine.current_state(), DreamState::Rem);
    }

    #[test]
    fn test_valid_transitions_rem_to_deep() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        let event = engine.transition_to(DreamState::Deep).unwrap();
        assert_eq!(event.from, DreamState::Rem);
        assert_eq!(event.to, DreamState::Deep);
    }

    #[test]
    fn test_valid_transitions_deep_to_lucid() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        let event = engine.transition_to(DreamState::Lucid).unwrap();
        assert_eq!(event.from, DreamState::Deep);
        assert_eq!(event.to, DreamState::Lucid);
    }

    #[test]
    fn test_valid_transition_back_to_waking() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        let event = engine.transition_to(DreamState::Waking).unwrap();
        assert_eq!(event.to, DreamState::Waking);
        assert!(!engine.is_dreaming());
    }

    #[test]
    fn test_invalid_transition_waking_to_deep() {
        let mut engine = CollectiveDreamingEngine::new();
        let result = engine.transition_to(DreamState::Deep);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_waking_to_lucid() {
        let mut engine = CollectiveDreamingEngine::new();
        let result = engine.transition_to(DreamState::Lucid);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_rem_to_lucid_skip() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        let result = engine.transition_to(DreamState::Lucid);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_transition_same_state() {
        let mut engine = CollectiveDreamingEngine::new();
        let result = engine.transition_to(DreamState::Waking);
        assert!(result.is_err());
    }

    // =========================================================================
    // Dreaming State Tests
    // =========================================================================

    #[test]
    fn test_is_dreaming_in_rem() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        assert!(engine.is_dreaming());
    }

    #[test]
    fn test_is_dreaming_in_deep() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        assert!(engine.is_dreaming());
    }

    #[test]
    fn test_is_dreaming_in_lucid() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        engine.transition_to(DreamState::Lucid).unwrap();
        assert!(engine.is_dreaming());
    }

    // =========================================================================
    // SAFEGUARD: Financial Transaction Blocking
    // =========================================================================

    #[test]
    fn test_financial_not_blocked_during_waking() {
        let engine = CollectiveDreamingEngine::new();
        assert!(!engine.is_financial_blocked());
        assert!(engine.guard_financial_operation("transfer").is_ok());
    }

    #[test]
    fn test_financial_blocked_during_rem() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        assert!(engine.is_financial_blocked());
        assert!(engine.guard_financial_operation("transfer").is_err());
    }

    #[test]
    fn test_financial_blocked_during_deep() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        assert!(engine.is_financial_blocked());
        assert!(engine.guard_financial_operation("stake").is_err());
    }

    #[test]
    fn test_financial_blocked_during_lucid() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        engine.transition_to(DreamState::Lucid).unwrap();
        assert!(engine.is_financial_blocked());
        assert!(engine.guard_financial_operation("withdraw").is_err());
    }

    #[test]
    fn test_financial_unblocked_after_returning_to_waking() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        assert!(engine.is_financial_blocked());

        engine.transition_to(DreamState::Waking).unwrap();
        assert!(!engine.is_financial_blocked());
        assert!(engine.guard_financial_operation("transfer").is_ok());
    }

    // =========================================================================
    // SAFEGUARD: Dream Proposal Non-Binding
    // =========================================================================

    #[test]
    fn test_proposal_created_as_non_binding() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Reorganize the trust network".to_string())
            .unwrap();

        assert!(!proposal.confirmed);
        assert!(!proposal.financial_operations);
        assert_eq!(proposal.confirmation_threshold, CONFIRMATION_THRESHOLD);
        assert_eq!(proposal.dream_state, DreamState::Rem);
    }

    #[test]
    fn test_proposal_rejected_during_waking() {
        let mut engine = CollectiveDreamingEngine::new();

        // Cannot submit during Waking
        let result = engine.submit_dream_proposal("Test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_proposal_financial_operations_always_false() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        for _ in 0..5 {
            let proposal = engine
                .submit_dream_proposal("Test proposal".to_string())
                .unwrap();
            // SAFEGUARD: financial_operations must ALWAYS be false
            assert!(!proposal.financial_operations);
        }
    }

    // =========================================================================
    // SAFEGUARD: Confirmation Threshold (0.67)
    // =========================================================================

    #[test]
    fn test_confirmation_passes_at_threshold() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();
        let proposal_id = proposal.id.clone();

        engine.transition_to(DreamState::Waking).unwrap();

        // 67 out of 100 = 0.67, exactly at threshold
        let confirmed = engine
            .confirm_dream_proposal(&proposal_id, 67, 100)
            .unwrap();
        assert!(confirmed);
        assert_eq!(engine.confirmed_proposal_count(), 1);
    }

    #[test]
    fn test_confirmation_fails_below_threshold() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();
        let proposal_id = proposal.id.clone();

        engine.transition_to(DreamState::Waking).unwrap();

        // 66 out of 100 = 0.66, below threshold
        let confirmed = engine
            .confirm_dream_proposal(&proposal_id, 66, 100)
            .unwrap();
        assert!(!confirmed);
        assert_eq!(engine.rejected_proposal_count(), 1);
        assert_eq!(engine.confirmed_proposal_count(), 0);
    }

    #[test]
    fn test_confirmation_passes_well_above_threshold() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();
        let proposal_id = proposal.id.clone();

        engine.transition_to(DreamState::Waking).unwrap();

        // Unanimous
        let confirmed = engine
            .confirm_dream_proposal(&proposal_id, 100, 100)
            .unwrap();
        assert!(confirmed);
    }

    #[test]
    fn test_confirmation_blocked_during_dream() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();

        // SAFEGUARD: Cannot confirm while dreaming
        let result = engine.confirm_dream_proposal(&proposal.id, 100, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_confirmation_zero_votes() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let proposal = engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();
        let proposal_id = proposal.id.clone();

        engine.transition_to(DreamState::Waking).unwrap();

        // Zero total votes -> rejected
        let confirmed = engine
            .confirm_dream_proposal(&proposal_id, 0, 0)
            .unwrap();
        assert!(!confirmed);
    }

    #[test]
    fn test_confirmation_nonexistent_proposal() {
        let mut engine = CollectiveDreamingEngine::new();
        let result = engine.confirm_dream_proposal("nonexistent", 10, 10);
        assert!(result.is_err());
    }

    // =========================================================================
    // Gate 1 Invariant Tests
    // =========================================================================

    #[test]
    fn test_gate1_all_pass_normal_state() {
        let engine = CollectiveDreamingEngine::new();
        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate1_all_pass_with_proposals() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine
            .submit_dream_proposal("Test 1".to_string())
            .unwrap();
        engine
            .submit_dream_proposal("Test 2".to_string())
            .unwrap();

        let checks = engine.gate1_check();
        // All proposals should have financial_operations=false, confirmed=false
        assert!(
            checks.iter().all(|c| c.passed),
            "Gate 1 violations: {:?}",
            checks.iter().filter(|c| !c.passed).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_gate1_financial_blocked_during_dream() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let checks = engine.gate1_check();
        let financial_check = checks
            .iter()
            .find(|c| c.invariant == "dream_financial_blocked_during_dreaming");
        assert!(financial_check.is_some());
        assert!(financial_check.unwrap().passed);
    }

    // =========================================================================
    // Transition History Tests
    // =========================================================================

    #[test]
    fn test_transition_history_recorded() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Deep).unwrap();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Waking).unwrap();

        let history = engine.transition_history();
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].0, DreamState::Waking);
        assert_eq!(history[0].1, DreamState::Rem);
        assert_eq!(history[3].0, DreamState::Rem);
        assert_eq!(history[3].1, DreamState::Waking);
    }

    // =========================================================================
    // Event Emission Tests
    // =========================================================================

    #[test]
    fn test_events_emitted_on_transition() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();

        let events = engine.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            LivingProtocolEvent::DreamStateChanged(e) => {
                assert_eq!(e.from, DreamState::Waking);
                assert_eq!(e.to, DreamState::Rem);
            }
            _ => panic!("Expected DreamStateChanged event"),
        }
    }

    #[test]
    fn test_events_emitted_on_proposal() {
        let mut engine = CollectiveDreamingEngine::new();
        engine.transition_to(DreamState::Rem).unwrap();
        engine.drain_events(); // clear transition event

        engine
            .submit_dream_proposal("Test".to_string())
            .unwrap();

        let events = engine.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            LivingProtocolEvent::DreamProposalGenerated(e) => {
                assert!(!e.proposal.confirmed);
                assert!(!e.proposal.financial_operations);
            }
            _ => panic!("Expected DreamProposalGenerated event"),
        }
    }

    // =========================================================================
    // LivingPrimitive Trait Tests
    // =========================================================================

    #[test]
    fn test_living_primitive_trait() {
        let engine = CollectiveDreamingEngine::new();
        assert_eq!(engine.primitive_id(), "collective_dreaming");
        assert_eq!(engine.primitive_number(), 7);
        assert_eq!(engine.module(), PrimitiveModule::Consciousness);
        assert_eq!(engine.tier(), 3);
    }

    #[test]
    fn test_collect_metrics() {
        let engine = CollectiveDreamingEngine::new();
        let metrics = engine.collect_metrics();
        assert_eq!(metrics["primitive"], "collective_dreaming");
        assert_eq!(metrics["current_state"], "Waking");
        assert_eq!(metrics["financial_blocked"], false);
    }

    // =========================================================================
    // Full Lifecycle Test
    // =========================================================================

    #[test]
    fn test_full_dream_lifecycle() {
        let mut engine = CollectiveDreamingEngine::new();

        // 1. Enter REM
        engine.transition_to(DreamState::Rem).unwrap();
        assert!(engine.is_dreaming());
        assert!(engine.is_financial_blocked());

        // 2. Submit proposals during REM
        let p1 = engine
            .submit_dream_proposal("Idea A: new trust metric".to_string())
            .unwrap();
        let p2 = engine
            .submit_dream_proposal("Idea B: governance reform".to_string())
            .unwrap();
        assert_eq!(engine.pending_proposal_count(), 2);

        // 3. Go deeper
        engine.transition_to(DreamState::Deep).unwrap();
        let p3 = engine
            .submit_dream_proposal("Deep insight: structural pattern".to_string())
            .unwrap();
        assert_eq!(engine.pending_proposal_count(), 3);

        // 4. Financial still blocked
        assert!(engine.guard_financial_operation("payment").is_err());

        // 5. Return through REM to Waking
        engine.transition_to(DreamState::Rem).unwrap();
        engine.transition_to(DreamState::Waking).unwrap();
        assert!(!engine.is_dreaming());
        assert!(!engine.is_financial_blocked());

        // 6. Confirm proposals during Waking
        let confirmed = engine
            .confirm_dream_proposal(&p1.id, 80, 100)
            .unwrap();
        assert!(confirmed); // 80% > 67%

        let confirmed = engine
            .confirm_dream_proposal(&p2.id, 60, 100)
            .unwrap();
        assert!(!confirmed); // 60% < 67%

        let confirmed = engine
            .confirm_dream_proposal(&p3.id, 67, 100)
            .unwrap();
        assert!(confirmed); // 67% = 67% (exact threshold)

        assert_eq!(engine.confirmed_proposal_count(), 2);
        assert_eq!(engine.rejected_proposal_count(), 1);
        assert_eq!(engine.pending_proposal_count(), 0);

        // 7. Financial operations now permitted
        assert!(engine.guard_financial_operation("payment").is_ok());
    }
}
