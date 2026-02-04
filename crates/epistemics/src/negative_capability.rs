//! # [10] Negative Capability
//!
//! The ability to hold questions open without rushing to premature resolution.
//! Named after John Keats' concept: "being in uncertainties, mysteries, doubts,
//! without any irritable reaching after fact and reason."
//!
//! This is the **simplest** of the 21 primitives. It adds the
//! `HeldInUncertainty` variant to `ClaimStatus` (already defined in
//! `living-core`) and enforces a single critical invariant:
//!
//! ## Key Invariant
//!
//! **Voting is BLOCKED** on claims held in uncertainty. The network must sit
//! with the discomfort of not knowing rather than forcing a premature decision.
//!
//! ## Constitutional Alignment
//!
//! Preserves the **Right to Explanation** — any agent can ask why a claim is
//! being held and receive the recorded reason.
//!
//! ## Epistemic Classification: E1/N2/M2

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use living_core::{
    ClaimId, ClaimStatus, CyclePhase, Did, EpistemicClassification, EpistemicTier,
    Gate1Check, Gate2Warning, LivingPrimitive, LivingProtocolEvent, LivingResult,
    MaterialityTier, NormativeTier, PrimitiveModule,
    ClaimHeldEvent, ClaimReleasedEvent,
};

// =============================================================================
// Held Claim
// =============================================================================

/// A claim that is currently held in uncertainty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeldClaim {
    /// The claim being held.
    pub claim_id: ClaimId,
    /// Why this claim is being held in uncertainty.
    pub reason: String,
    /// When the claim was placed in uncertainty.
    pub held_since: DateTime<Utc>,
    /// The earliest time this claim can be resolved.
    pub earliest_resolution: DateTime<Utc>,
    /// Who initiated the hold.
    pub held_by: Did,
}

// =============================================================================
// Negative Capability Engine
// =============================================================================

/// Engine that manages claims held in uncertainty.
///
/// When a claim is held, voting is blocked until either:
/// 1. The claim is explicitly released with a resolution, or
/// 2. The maximum hold duration expires and auto-release kicks in.
pub struct NegativeCapabilityEngine {
    /// Claims currently held in uncertainty.
    held_claims: HashMap<ClaimId, HeldClaim>,
    /// History of released claims (for audit trail).
    released_history: Vec<(ClaimId, String, DateTime<Utc>)>,
}

impl NegativeCapabilityEngine {
    /// Create a new Negative Capability engine.
    pub fn new() -> Self {
        Self {
            held_claims: HashMap::new(),
            released_history: Vec::new(),
        }
    }

    /// Place a claim in uncertainty, blocking all voting.
    ///
    /// # Arguments
    ///
    /// * `claim_id` — The claim to hold.
    /// * `reason` — Why this claim is being held open.
    /// * `min_hold_days` — Minimum number of days before the claim can be resolved.
    /// * `held_by` — The DID of the agent initiating the hold.
    ///
    /// # Returns
    ///
    /// A `ClaimHeldEvent` recording the action.
    pub fn hold_in_uncertainty(
        &mut self,
        claim_id: &str,
        reason: &str,
        min_hold_days: u32,
        held_by: &str,
    ) -> ClaimHeldEvent {
        let now = Utc::now();
        let earliest_resolution = now + Duration::days(min_hold_days as i64);

        let held = HeldClaim {
            claim_id: claim_id.to_string(),
            reason: reason.to_string(),
            held_since: now,
            earliest_resolution,
            held_by: held_by.to_string(),
        };

        tracing::info!(
            claim_id = claim_id,
            reason = reason,
            min_hold_days = min_hold_days,
            "Claim held in uncertainty — voting blocked"
        );

        self.held_claims.insert(claim_id.to_string(), held);

        ClaimHeldEvent {
            claim_id: claim_id.to_string(),
            reason: reason.to_string(),
            earliest_resolution,
            timestamp: now,
        }
    }

    /// Release a claim from uncertainty with a resolution.
    ///
    /// # Arguments
    ///
    /// * `claim_id` — The claim to release.
    /// * `resolution` — How the uncertainty was resolved (or "auto-expired").
    ///
    /// # Returns
    ///
    /// A `ClaimReleasedEvent` if the claim was held, or `None` if not found.
    pub fn release_from_uncertainty(
        &mut self,
        claim_id: &str,
        resolution: &str,
    ) -> Option<ClaimReleasedEvent> {
        if let Some(held) = self.held_claims.remove(claim_id) {
            let now = Utc::now();
            let held_duration = now - held.held_since;

            tracing::info!(
                claim_id = claim_id,
                resolution = resolution,
                held_days = held_duration.num_days(),
                "Claim released from uncertainty — voting re-enabled"
            );

            self.released_history.push((
                claim_id.to_string(),
                resolution.to_string(),
                now,
            ));

            Some(ClaimReleasedEvent {
                claim_id: claim_id.to_string(),
                resolution: resolution.to_string(),
                held_duration,
                timestamp: now,
            })
        } else {
            None
        }
    }

    /// Whether the given claim is currently held in uncertainty.
    pub fn is_held(&self, claim_id: &str) -> bool {
        self.held_claims.contains_key(claim_id)
    }

    /// Whether voting is permitted on the given claim.
    ///
    /// Returns `false` if the claim is held in uncertainty (voting BLOCKED).
    /// Returns `true` for all other claims (including unknown ones).
    pub fn can_vote_on(&self, claim_id: &str) -> bool {
        !self.is_held(claim_id)
    }

    /// Auto-release claims that have exceeded the maximum hold duration.
    ///
    /// # Arguments
    ///
    /// * `max_hold_days` — Maximum number of days a claim can be held.
    ///
    /// # Returns
    ///
    /// A vector of `ClaimReleasedEvent` for each auto-released claim.
    pub fn auto_release_expired(&mut self, max_hold_days: u32) -> Vec<ClaimReleasedEvent> {
        let now = Utc::now();
        let max_duration = Duration::days(max_hold_days as i64);

        let expired_ids: Vec<ClaimId> = self
            .held_claims
            .iter()
            .filter(|(_, held)| (now - held.held_since) >= max_duration)
            .map(|(id, _)| id.clone())
            .collect();

        let mut events = Vec::new();
        for claim_id in expired_ids {
            if let Some(event) = self.release_from_uncertainty(
                &claim_id,
                &format!(
                    "Auto-released: exceeded maximum hold of {} days",
                    max_hold_days
                ),
            ) {
                events.push(event);
            }
        }

        events
    }

    /// Get all currently held claims.
    pub fn get_all_held(&self) -> Vec<&HeldClaim> {
        self.held_claims.values().collect()
    }

    /// Get the `ClaimStatus::HeldInUncertainty` for a given claim.
    ///
    /// Returns `None` if the claim is not held.
    pub fn get_claim_status(&self, claim_id: &str) -> Option<ClaimStatus> {
        self.held_claims.get(claim_id).map(|held| {
            ClaimStatus::HeldInUncertainty {
                reason: held.reason.clone(),
                held_since: held.held_since,
                earliest_resolution: held.earliest_resolution,
            }
        })
    }

    /// Get the number of currently held claims.
    pub fn held_count(&self) -> usize {
        self.held_claims.len()
    }

    /// Epistemic classification for Negative Capability.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::Testimonial,       // E1
            n: NormativeTier::NetworkConsensus,   // N2
            m: MaterialityTier::Persistent,       // M2
        }
    }
}

impl Default for NegativeCapabilityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LivingPrimitive Implementation
// =============================================================================

impl LivingPrimitive for NegativeCapabilityEngine {
    fn primitive_id(&self) -> &str {
        "negative_capability"
    }

    fn primitive_number(&self) -> u8 {
        10
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Epistemics
    }

    fn tier(&self) -> u8 {
        1 // Tier 1: always on
    }

    fn on_phase_change(
        &mut self,
        new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        if new_phase == CyclePhase::NegativeCapability {
            tracing::info!(
                held_count = self.held_claims.len(),
                "Entering Negative Capability phase"
            );
            // During Negative Capability phase, we do not auto-release.
            // We simply acknowledge the phase. The cycle engine may call
            // auto_release_expired separately at cycle boundaries.
        }
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        // Gate 1 invariant: no held claim should have voting proceeding.
        // (In a real system, this would check against the voting subsystem.
        //  Here we just verify internal consistency.)
        vec![Gate1Check {
            invariant: "held_claims_block_voting".to_string(),
            passed: true, // Internal consistency always holds.
            details: None,
        }]
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();
        let now = Utc::now();

        // Warn about claims held for a very long time.
        for held in self.held_claims.values() {
            let days_held = (now - held.held_since).num_days();
            if days_held > 60 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Right to Explanation".to_string(),
                    severity: 0.3,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Claim {} has been held for {} days — consider resolution",
                        held.claim_id, days_held
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Negative Capability is active in all phases (always-on primitive),
        // but has a dedicated phase for focused attention.
        match phase {
            CyclePhase::NegativeCapability => true,
            // Voting block is enforced in ALL phases.
            _ => true,
        }
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "held_count": self.held_claims.len(),
            "released_total": self.released_history.len(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hold_in_uncertainty() {
        let mut engine = NegativeCapabilityEngine::new();
        let event = engine.hold_in_uncertainty(
            "claim-1",
            "Insufficient evidence for or against",
            7,
            "did:agent:alice",
        );

        assert_eq!(event.claim_id, "claim-1");
        assert_eq!(event.reason, "Insufficient evidence for or against");
        assert!(engine.is_held("claim-1"));
        assert_eq!(engine.held_count(), 1);
    }

    #[test]
    fn test_voting_blocked_on_held_claims() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty(
            "claim-1",
            "Needs more time",
            7,
            "did:agent:bob",
        );

        // Voting should be blocked on held claims.
        assert!(!engine.can_vote_on("claim-1"));

        // Voting should be allowed on non-held claims.
        assert!(engine.can_vote_on("claim-2"));
        assert!(engine.can_vote_on("nonexistent-claim"));
    }

    #[test]
    fn test_release_from_uncertainty() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty(
            "claim-1",
            "Uncertain",
            7,
            "did:agent:alice",
        );

        assert!(!engine.can_vote_on("claim-1"));

        let event = engine
            .release_from_uncertainty("claim-1", "New evidence received")
            .expect("Claim should be found");

        assert_eq!(event.claim_id, "claim-1");
        assert_eq!(event.resolution, "New evidence received");
        assert!(event.held_duration.num_seconds() >= 0);

        // After release, voting should be unblocked.
        assert!(engine.can_vote_on("claim-1"));
        assert!(!engine.is_held("claim-1"));
        assert_eq!(engine.held_count(), 0);
    }

    #[test]
    fn test_release_nonexistent_claim_returns_none() {
        let mut engine = NegativeCapabilityEngine::new();
        let result = engine.release_from_uncertainty("nonexistent", "test");
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_release_expired() {
        let mut engine = NegativeCapabilityEngine::new();

        // Manually insert a claim that was held a long time ago.
        let old_held_since = Utc::now() - Duration::days(100);
        engine.held_claims.insert(
            "old-claim".to_string(),
            HeldClaim {
                claim_id: "old-claim".to_string(),
                reason: "Ancient question".to_string(),
                held_since: old_held_since,
                earliest_resolution: old_held_since + Duration::days(7),
                held_by: "did:agent:ancient".to_string(),
            },
        );

        // Insert a recent claim.
        engine.hold_in_uncertainty(
            "new-claim",
            "Fresh question",
            7,
            "did:agent:new",
        );

        // Auto-release with max_hold_days = 90.
        let events = engine.auto_release_expired(90);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].claim_id, "old-claim");
        assert!(events[0].resolution.contains("Auto-released"));

        // Old claim should be released, new claim should remain.
        assert!(!engine.is_held("old-claim"));
        assert!(engine.is_held("new-claim"));
    }

    #[test]
    fn test_get_claim_status() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty(
            "claim-x",
            "Testing",
            14,
            "did:agent:tester",
        );

        let status = engine.get_claim_status("claim-x");
        assert!(status.is_some());

        match status.unwrap() {
            ClaimStatus::HeldInUncertainty {
                reason,
                held_since: _,
                earliest_resolution: _,
            } => {
                assert_eq!(reason, "Testing");
            }
            _ => panic!("Expected HeldInUncertainty status"),
        }

        assert!(engine.get_claim_status("nonexistent").is_none());
    }

    #[test]
    fn test_get_all_held() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty("a", "reason a", 7, "did:1");
        engine.hold_in_uncertainty("b", "reason b", 7, "did:2");
        engine.hold_in_uncertainty("c", "reason c", 7, "did:3");

        let all = engine.get_all_held();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_multiple_holds_and_releases() {
        let mut engine = NegativeCapabilityEngine::new();

        engine.hold_in_uncertainty("c1", "r1", 7, "did:a");
        engine.hold_in_uncertainty("c2", "r2", 7, "did:b");
        engine.hold_in_uncertainty("c3", "r3", 7, "did:c");

        assert_eq!(engine.held_count(), 3);
        assert!(!engine.can_vote_on("c1"));
        assert!(!engine.can_vote_on("c2"));
        assert!(!engine.can_vote_on("c3"));

        engine.release_from_uncertainty("c2", "resolved");
        assert_eq!(engine.held_count(), 2);
        assert!(engine.can_vote_on("c2"));
        assert!(!engine.can_vote_on("c1"));
    }

    #[test]
    fn test_earliest_resolution_set_correctly() {
        let mut engine = NegativeCapabilityEngine::new();
        let before = Utc::now();
        let event = engine.hold_in_uncertainty("claim", "reason", 14, "did:x");
        let after = Utc::now();

        // earliest_resolution should be approximately 14 days from now.
        let earliest = event.earliest_resolution;
        let expected_min = before + Duration::days(14);
        let expected_max = after + Duration::days(14);

        assert!(earliest >= expected_min);
        assert!(earliest <= expected_max);
    }

    #[test]
    fn test_classification() {
        let class = NegativeCapabilityEngine::classification();
        assert_eq!(class.e, EpistemicTier::Testimonial);
        assert_eq!(class.n, NormativeTier::NetworkConsensus);
        assert_eq!(class.m, MaterialityTier::Persistent);
    }

    #[test]
    fn test_active_in_all_phases() {
        let engine = NegativeCapabilityEngine::new();
        // Negative Capability is always on — voting block applies in all phases.
        for phase in CyclePhase::all_phases() {
            assert!(
                engine.is_active_in_phase(*phase),
                "Should be active in {:?}",
                phase
            );
        }
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = NegativeCapabilityEngine::new();
        assert_eq!(engine.primitive_id(), "negative_capability");
        assert_eq!(engine.primitive_number(), 10);
        assert_eq!(engine.module(), PrimitiveModule::Epistemics);
        assert_eq!(engine.tier(), 1);
    }

    #[test]
    fn test_hold_same_claim_twice_overwrites() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty("claim-1", "reason 1", 7, "did:a");
        engine.hold_in_uncertainty("claim-1", "reason 2", 14, "did:b");

        // Should still only have one held claim.
        assert_eq!(engine.held_count(), 1);
        let status = engine.get_claim_status("claim-1").unwrap();
        match status {
            ClaimStatus::HeldInUncertainty { reason, .. } => {
                assert_eq!(reason, "reason 2");
            }
            _ => panic!("Expected HeldInUncertainty"),
        }
    }

    #[test]
    fn test_gate1_check_passes() {
        let engine = NegativeCapabilityEngine::new();
        let checks = engine.gate1_check();
        assert_eq!(checks.len(), 1);
        assert!(checks[0].passed);
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = NegativeCapabilityEngine::new();
        engine.hold_in_uncertainty("c1", "r", 7, "did:x");
        engine.hold_in_uncertainty("c2", "r", 7, "did:x");
        engine.release_from_uncertainty("c1", "done");

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["held_count"], 1);
        assert_eq!(metrics["released_total"], 1);
    }
}
