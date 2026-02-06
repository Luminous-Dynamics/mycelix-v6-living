//! # [9] Shadow Integration
//!
//! Periodic surfacing of suppressed dissent. During the Shadow phase of the
//! Metabolism Cycle, the engine examines content that was downvoted or
//! suppressed and selectively surfaces it for re-evaluation.
//!
//! ## Key Invariant
//!
//! Shadow Integration **NEVER** surfaces Gate 1-protected content
//! (cryptographic violations, mathematical invariant breaches, etc.).
//!
//! ## Spectral K Trigger
//!
//! Low Spectral K (small algebraic connectivity lambda-2) suggests the network
//! is fragmenting or that groupthink is suppressing dissent. When spectral_k
//! falls below the configured anomaly threshold, the shadow phase is triggered
//! more aggressively.
//!
//! ## Constitutional Alignment
//!
//! Preserves the **Right to Epistemic Humility** — ensures the network does
//! not prematurely discard minority viewpoints.
//!
//! ## Epistemic Classification: E2/N2/M1

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use living_core::{
    CyclePhase, EpistemicClassification, EpistemicTier, Gate1Check, Gate2Warning, LivingPrimitive,
    LivingProtocolEvent, LivingResult, MaterialityTier, NormativeTier, PrimitiveModule,
    ShadowConfig, ShadowRecord, ShadowSurfacedEvent,
};

// =============================================================================
// Suppressed Content
// =============================================================================

/// Content that was suppressed (downvoted, flagged, or otherwise hidden).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressedContent {
    /// Unique identifier for this suppressed item.
    pub id: String,
    /// The content identifier in the broader system.
    pub content_id: String,
    /// Why this content was suppressed.
    pub reason: String,
    /// Reputation of the original author at the time of suppression.
    pub author_reputation: f64,
    /// Reputation of the suppressor at the time of suppression.
    pub suppressor_reputation: f64,
    /// When suppression occurred.
    pub suppressed_at: DateTime<Utc>,
    /// Whether this content is protected by Gate 1 (cryptographic invariant).
    /// If true, it must NEVER be surfaced.
    pub gate1_protected: bool,
}

// =============================================================================
// Shadow Integration Engine
// =============================================================================

/// Engine that manages the Shadow Integration process.
///
/// During the Shadow phase of the Metabolism Cycle, this engine reviews
/// suppressed content and surfaces items that deserve re-examination —
/// particularly low-reputation dissent that may have been drowned out by
/// majority groupthink.
pub struct ShadowIntegrationEngine {
    /// All suppressed content awaiting potential surfacing.
    suppressed_content: Vec<SuppressedContent>,
    /// Content that has already been surfaced.
    surfaced: Vec<ShadowRecord>,
    /// Content IDs that are Gate 1-protected and must never be surfaced.
    gate1_protected_ids: HashSet<String>,
}

impl ShadowIntegrationEngine {
    /// Create a new Shadow Integration engine.
    pub fn new() -> Self {
        Self {
            suppressed_content: Vec::new(),
            surfaced: Vec::new(),
            gate1_protected_ids: HashSet::new(),
        }
    }

    /// Record that content has been suppressed.
    ///
    /// Returns the unique suppression record ID.
    pub fn record_suppression(
        &mut self,
        content_id: &str,
        reason: &str,
        suppressor_rep: f64,
        author_rep: f64,
        gate1_protected: bool,
    ) -> String {
        let id = Uuid::new_v4().to_string();

        if gate1_protected {
            self.gate1_protected_ids.insert(content_id.to_string());
        }

        let record = SuppressedContent {
            id: id.clone(),
            content_id: content_id.to_string(),
            reason: reason.to_string(),
            author_reputation: author_rep,
            suppressor_reputation: suppressor_rep,
            suppressed_at: Utc::now(),
            gate1_protected,
        };

        tracing::info!(
            content_id = content_id,
            gate1_protected = gate1_protected,
            "Recorded suppression of content"
        );

        self.suppressed_content.push(record);
        id
    }

    /// Run the Shadow phase: surface suppressed content that deserves review.
    ///
    /// # Arguments
    ///
    /// * `spectral_k` — Current spectral gap (lambda-2). Low values indicate
    ///   potential groupthink and trigger more aggressive surfacing.
    /// * `config` — Shadow configuration parameters.
    ///
    /// # Returns
    ///
    /// A vector of `ShadowSurfacedEvent` for each item surfaced.
    pub fn run_shadow_phase(
        &mut self,
        spectral_k: f64,
        config: &ShadowConfig,
    ) -> Vec<ShadowSurfacedEvent> {
        let mut events = Vec::new();
        let now = Utc::now();

        // Determine the anomaly score. A low spectral_k relative to the
        // threshold means the network is more fragmented / groupthink-prone.
        let anomaly = if spectral_k < config.spectral_k_anomaly_threshold {
            Some(config.spectral_k_anomaly_threshold - spectral_k)
        } else {
            None
        };

        // Sort candidates: prioritize low-reputation dissent first (ascending
        // author reputation), then oldest suppressions.
        let mut candidates: Vec<&SuppressedContent> = self
            .suppressed_content
            .iter()
            .filter(|s| !s.gate1_protected)
            .collect();

        candidates.sort_by(|a, b| {
            a.author_reputation
                .partial_cmp(&b.author_reputation)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.suppressed_at.cmp(&b.suppressed_at))
        });

        // How many to surface this phase.
        let surface_count = if anomaly.is_some() {
            // During anomaly, surface up to the max.
            config.max_surface_per_phase
        } else {
            // Normal surfacing: half the max, minimum 1 if there are candidates.
            (config.max_surface_per_phase / 2).max(1)
        };

        let to_surface: Vec<String> = candidates
            .iter()
            .take(surface_count)
            .map(|c| c.content_id.clone())
            .collect();

        for content_id in &to_surface {
            // Find the suppressed content record.
            if let Some(suppressed) = self
                .suppressed_content
                .iter()
                .find(|s| &s.content_id == content_id)
            {
                let shadow_record = ShadowRecord {
                    id: Uuid::new_v4().to_string(),
                    original_content_id: content_id.clone(),
                    suppressed_at: suppressed.suppressed_at,
                    surfaced_at: now,
                    suppression_reason: suppressed.reason.clone(),
                    low_rep_dissent: suppressed.author_reputation < 0.3,
                    spectral_k_anomaly: anomaly,
                };

                let event = ShadowSurfacedEvent {
                    shadow: shadow_record.clone(),
                    timestamp: now,
                };

                tracing::info!(
                    content_id = content_id.as_str(),
                    low_rep_dissent = shadow_record.low_rep_dissent,
                    spectral_k = spectral_k,
                    "Surfaced shadow content"
                );

                self.surfaced.push(shadow_record);
                events.push(event);
            }
        }

        // Remove surfaced items from the suppressed list.
        let surfaced_set: HashSet<String> = to_surface.into_iter().collect();
        self.suppressed_content
            .retain(|s| !surfaced_set.contains(&s.content_id));

        events
    }

    /// Get all surfaced shadow records.
    pub fn get_surfaced_shadows(&self) -> &[ShadowRecord] {
        &self.surfaced
    }

    /// Get all currently suppressed content.
    pub fn get_suppressed_content(&self) -> &[SuppressedContent] {
        &self.suppressed_content
    }

    /// Check whether the given content ID is Gate 1-protected.
    ///
    /// Gate 1-protected content must NEVER be surfaced. This includes content
    /// that violates cryptographic invariants, mathematical proofs of
    /// invalidity, etc.
    pub fn is_gate1_protected(&self, content_id: &str) -> bool {
        self.gate1_protected_ids.contains(content_id)
    }

    /// Mark a content ID as Gate 1-protected (e.g., discovered after initial
    /// suppression that it violates a cryptographic invariant).
    pub fn mark_gate1_protected(&mut self, content_id: &str) {
        self.gate1_protected_ids.insert(content_id.to_string());
        // Also mark any existing suppressed content record.
        for item in &mut self.suppressed_content {
            if item.content_id == content_id {
                item.gate1_protected = true;
            }
        }
    }

    /// Epistemic classification for Shadow Integration.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::PrivatelyVerifiable, // E2
            n: NormativeTier::NetworkConsensus,    // N2
            m: MaterialityTier::Temporal,          // M1
        }
    }
}

impl Default for ShadowIntegrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LivingPrimitive Implementation
// =============================================================================

impl LivingPrimitive for ShadowIntegrationEngine {
    fn primitive_id(&self) -> &str {
        "shadow_integration"
    }

    fn primitive_number(&self) -> u8 {
        9
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Epistemics
    }

    fn tier(&self) -> u8 {
        2 // Tier 2: default on
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        if new_phase == CyclePhase::Shadow {
            // Use default config and a neutral spectral_k.
            let config = ShadowConfig::default();
            let events = self.run_shadow_phase(config.spectral_k_anomaly_threshold, &config);
            Ok(events
                .into_iter()
                .map(LivingProtocolEvent::ShadowSurfaced)
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        // Gate 1 invariant: no Gate 1-protected content has been surfaced.
        let violated = self
            .surfaced
            .iter()
            .any(|s| self.gate1_protected_ids.contains(&s.original_content_id));

        vec![Gate1Check {
            invariant: "shadow_never_surfaces_gate1_protected".to_string(),
            passed: !violated,
            details: if violated {
                Some("Gate 1-protected content was surfaced — critical invariant violation".into())
            } else {
                None
            },
        }]
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Warn if there is a large backlog of suppressed content.
        if self.suppressed_content.len() > 100 {
            warnings.push(Gate2Warning {
                harmony_violated: "Right to Epistemic Humility".to_string(),
                severity: 0.4,
                reputation_impact: 0.0,
                reasoning: format!(
                    "Large suppressed content backlog ({} items) may indicate systemic silencing",
                    self.suppressed_content.len()
                ),
                user_may_proceed: true,
            });
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        phase == CyclePhase::Shadow
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "suppressed_count": self.suppressed_content.len(),
            "surfaced_count": self.surfaced.len(),
            "gate1_protected_count": self.gate1_protected_ids.len(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::ShadowConfig;

    fn default_config() -> ShadowConfig {
        ShadowConfig::default()
    }

    #[test]
    fn test_record_suppression() {
        let mut engine = ShadowIntegrationEngine::new();
        let id = engine.record_suppression("content-1", "spam", 0.8, 0.2, false);
        assert!(!id.is_empty());
        assert_eq!(engine.get_suppressed_content().len(), 1);
    }

    #[test]
    fn test_gate1_protection_blocks_surfacing() {
        let mut engine = ShadowIntegrationEngine::new();

        // Suppress two items: one Gate 1-protected, one not.
        engine.record_suppression("crypto-violation", "invalid sig", 0.9, 0.1, true);
        engine.record_suppression("legitimate-dissent", "minority view", 0.8, 0.15, false);

        assert!(engine.is_gate1_protected("crypto-violation"));
        assert!(!engine.is_gate1_protected("legitimate-dissent"));

        let config = default_config();
        let events = engine.run_shadow_phase(0.1, &config); // low spectral_k

        // Only the non-protected content should be surfaced.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].shadow.original_content_id, "legitimate-dissent");

        // Verify the Gate 1 check passes.
        let checks = engine.gate1_check();
        assert!(checks[0].passed, "Gate 1 check should pass");
    }

    #[test]
    fn test_gate1_protection_invariant_never_violated() {
        let mut engine = ShadowIntegrationEngine::new();

        // Add only Gate 1-protected content.
        engine.record_suppression("bad-1", "crypto fail", 0.9, 0.1, true);
        engine.record_suppression("bad-2", "sig invalid", 0.8, 0.05, true);

        let config = default_config();
        let events = engine.run_shadow_phase(0.05, &config);

        // Nothing should be surfaced.
        assert!(events.is_empty());
        assert!(engine.gate1_check()[0].passed);
    }

    #[test]
    fn test_mark_gate1_protected_after_suppression() {
        let mut engine = ShadowIntegrationEngine::new();

        engine.record_suppression("content-x", "initially ok", 0.7, 0.3, false);
        assert!(!engine.is_gate1_protected("content-x"));

        // Later discovery: this content actually violates Gate 1.
        engine.mark_gate1_protected("content-x");
        assert!(engine.is_gate1_protected("content-x"));

        let config = default_config();
        let events = engine.run_shadow_phase(0.1, &config);
        assert!(events.is_empty());
    }

    #[test]
    fn test_low_rep_dissent_prioritized() {
        let mut engine = ShadowIntegrationEngine::new();

        // High-rep author suppressed content.
        engine.record_suppression("high-rep-content", "disagreement", 0.7, 0.8, false);
        // Low-rep author suppressed content.
        engine.record_suppression("low-rep-content", "minority view", 0.9, 0.1, false);

        let mut config = default_config();
        config.max_surface_per_phase = 1; // Only surface 1 item.

        let events = engine.run_shadow_phase(0.1, &config);
        assert_eq!(events.len(), 1);
        // Low-rep content should be surfaced first.
        assert_eq!(events[0].shadow.original_content_id, "low-rep-content");
        assert!(events[0].shadow.low_rep_dissent);
    }

    #[test]
    fn test_spectral_k_anomaly_triggers_more_surfacing() {
        let mut engine = ShadowIntegrationEngine::new();

        for i in 0..20 {
            engine.record_suppression(&format!("content-{}", i), "suppressed", 0.8, 0.3, false);
        }

        let config = default_config(); // max_surface_per_phase = 10

        // Normal spectral_k: surface half of max (5, minimum 1).
        let events_normal =
            engine.run_shadow_phase(config.spectral_k_anomaly_threshold + 0.1, &config);
        let normal_count = events_normal.len();

        // Add more content back.
        for i in 20..40 {
            engine.record_suppression(&format!("content-{}", i), "suppressed", 0.8, 0.3, false);
        }

        // Low spectral_k (anomaly): surface full max.
        let events_anomaly = engine.run_shadow_phase(0.05, &config);
        let anomaly_count = events_anomaly.len();

        assert!(
            anomaly_count >= normal_count,
            "Anomaly should surface at least as much: anomaly={}, normal={}",
            anomaly_count,
            normal_count,
        );
    }

    #[test]
    fn test_surfaced_content_removed_from_suppressed() {
        let mut engine = ShadowIntegrationEngine::new();
        engine.record_suppression("content-1", "reason", 0.5, 0.3, false);

        assert_eq!(engine.get_suppressed_content().len(), 1);

        let config = default_config();
        let events = engine.run_shadow_phase(0.1, &config);
        assert_eq!(events.len(), 1);

        // After surfacing, the item should be removed from suppressed.
        assert_eq!(engine.get_suppressed_content().len(), 0);
        assert_eq!(engine.get_surfaced_shadows().len(), 1);
    }

    #[test]
    fn test_classification() {
        let class = ShadowIntegrationEngine::classification();
        assert_eq!(class.e, EpistemicTier::PrivatelyVerifiable);
        assert_eq!(class.n, NormativeTier::NetworkConsensus);
        assert_eq!(class.m, MaterialityTier::Temporal);
    }

    #[test]
    fn test_is_active_in_shadow_phase_only() {
        let engine = ShadowIntegrationEngine::new();
        assert!(engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::Beauty));
        assert!(!engine.is_active_in_phase(CyclePhase::NegativeCapability));
        assert!(!engine.is_active_in_phase(CyclePhase::CoCreation));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = ShadowIntegrationEngine::new();
        assert_eq!(engine.primitive_id(), "shadow_integration");
        assert_eq!(engine.primitive_number(), 9);
        assert_eq!(engine.module(), PrimitiveModule::Epistemics);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_empty_suppressed_list() {
        let mut engine = ShadowIntegrationEngine::new();
        let config = default_config();
        let events = engine.run_shadow_phase(0.1, &config);
        assert!(events.is_empty());
    }

    #[test]
    fn test_spectral_k_anomaly_recorded_in_shadow_record() {
        let mut engine = ShadowIntegrationEngine::new();
        engine.record_suppression("content-a", "test", 0.5, 0.2, false);

        let config = default_config();
        // Spectral K below threshold => anomaly.
        let events = engine.run_shadow_phase(0.1, &config);
        assert_eq!(events.len(), 1);
        assert!(events[0].shadow.spectral_k_anomaly.is_some());
        let anomaly_val = events[0].shadow.spectral_k_anomaly.unwrap();
        assert!(anomaly_val > 0.0);
    }

    #[test]
    fn test_no_anomaly_when_spectral_k_above_threshold() {
        let mut engine = ShadowIntegrationEngine::new();
        engine.record_suppression("content-b", "test", 0.5, 0.5, false);

        let config = default_config();
        let events = engine.run_shadow_phase(0.5, &config);
        assert_eq!(events.len(), 1);
        assert!(events[0].shadow.spectral_k_anomaly.is_none());
    }
}
