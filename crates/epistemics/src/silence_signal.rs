//! # [11] Silence as Signal
//!
//! Detects and classifies meaningful absence. When an agent is provably
//! present (via `PresenceProof` heartbeats) but silent on a topic, that
//! silence itself carries epistemic weight.
//!
//! ## Key Invariant
//!
//! Silence requires a **valid PresenceProof**. You cannot fake presence to
//! manufacture the appearance of meaningful silence. Without heartbeats,
//! an agent is simply absent — not silently dissenting.
//!
//! ## Silence Classifications
//!
//! - `DeliberateWithholding` — Agent chooses not to speak.
//! - `Contemplative` — Agent is processing / thinking.
//! - `DissentThroughAbsence` — Agent disagrees but avoids conflict.
//! - `Unknown` — Insufficient data to classify.
//!
//! ## Constitutional Alignment
//!
//! Integral Wisdom (Harmony 3) — recognizes that what is *not* said may be
//! as important as what is said.
//!
//! ## Epistemic Classification: E2/N1/M1

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use living_core::{
    CyclePhase, Did, EpistemicClassification, EpistemicTier, Gate1Check, Gate2Warning,
    LivingPrimitive, LivingProtocolEvent, LivingResult, MaterialityTier, NormativeTier,
    PresenceProof, PrimitiveModule, SilenceClassification, SilenceDetectedEvent, SilenceRecord,
};

// =============================================================================
// Presence Status
// =============================================================================

/// Current presence status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceStatus {
    /// Agent has recent heartbeats and is participating in discussion.
    Active,
    /// Agent has recent heartbeats but is not speaking on one or more topics.
    Silent,
    /// Agent has no recent heartbeats — not provably present.
    Absent,
}

// =============================================================================
// Topic Activity Tracker
// =============================================================================

/// Tracks which agents have spoken on which topics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicActivity {
    /// Topic identifier.
    pub topic: String,
    /// Set of agent DIDs that have spoken on this topic.
    pub speakers: Vec<Did>,
    /// When the topic became active.
    pub active_since: DateTime<Utc>,
}

// =============================================================================
// Silence Signal Engine
// =============================================================================

/// Engine that detects and classifies meaningful silence.
///
/// Maintains a presence registry of heartbeats and compares agent presence
/// against topic participation to identify meaningful silences.
pub struct SilenceSignalEngine {
    /// Presence proofs per agent, ordered by timestamp.
    presence_registry: HashMap<Did, Vec<PresenceProof>>,
    /// Detected silence records.
    silence_records: Vec<SilenceRecord>,
    /// Active topics and their speakers.
    topic_activity: HashMap<String, TopicActivity>,
    /// Maximum age of heartbeats before they expire (hours).
    heartbeat_expiry_hours: i64,
}

impl SilenceSignalEngine {
    /// Create a new Silence Signal engine.
    pub fn new() -> Self {
        Self {
            presence_registry: HashMap::new(),
            silence_records: Vec::new(),
            topic_activity: HashMap::new(),
            heartbeat_expiry_hours: 48,
        }
    }

    /// Create with custom heartbeat expiry.
    pub fn with_expiry(heartbeat_expiry_hours: i64) -> Self {
        Self {
            heartbeat_expiry_hours,
            ..Self::new()
        }
    }

    /// Submit a heartbeat proving an agent is present.
    ///
    /// The heartbeat is validated before being recorded.
    ///
    /// Returns `true` if the heartbeat was accepted, `false` if it failed
    /// validation.
    pub fn submit_heartbeat(&mut self, agent_did: &str, proof: PresenceProof) -> bool {
        if !self.validate_presence_proof(&proof) {
            tracing::warn!(agent_did = agent_did, "Rejected invalid PresenceProof");
            return false;
        }

        if proof.agent_did != agent_did {
            tracing::warn!(
                expected = agent_did,
                actual = proof.agent_did.as_str(),
                "PresenceProof agent_did mismatch"
            );
            return false;
        }

        let proofs = self
            .presence_registry
            .entry(agent_did.to_string())
            .or_default();

        proofs.push(proof);

        // Keep only recent heartbeats to bound memory.
        let cutoff = Utc::now() - Duration::hours(self.heartbeat_expiry_hours);
        proofs.retain(|p| p.timestamp > cutoff);

        true
    }

    /// Validate a PresenceProof.
    ///
    /// Checks:
    /// 1. The heartbeat hash is non-zero (not a null proof).
    /// 2. The signature is non-empty.
    /// 3. The timestamp is not in the future.
    /// 4. The heartbeat hash matches the expected structure (SHA-256 of
    ///    `agent_did || timestamp`).
    ///
    /// Note: In production, the signature would be verified against the
    /// agent's public key. Here we validate structural correctness.
    pub fn validate_presence_proof(&self, proof: &PresenceProof) -> bool {
        // Check non-null hash.
        if proof.heartbeat_hash == [0u8; 32] {
            return false;
        }

        // Check signature is present.
        if proof.signature.is_empty() {
            return false;
        }

        // Check timestamp is not in the future (with small tolerance).
        let now = Utc::now();
        if proof.timestamp > now + Duration::seconds(60) {
            return false;
        }

        // Verify the heartbeat hash structure.
        let expected = Self::compute_heartbeat_hash(&proof.agent_did, proof.timestamp);
        if proof.heartbeat_hash != expected {
            return false;
        }

        true
    }

    /// Compute the expected heartbeat hash for an agent at a given time.
    pub fn compute_heartbeat_hash(agent_did: &str, timestamp: DateTime<Utc>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(agent_did.as_bytes());
        hasher.update(timestamp.timestamp().to_le_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Record that an agent has spoken on a topic.
    pub fn record_speech(&mut self, agent_did: &str, topic: &str) {
        let activity = self
            .topic_activity
            .entry(topic.to_string())
            .or_insert_with(|| TopicActivity {
                topic: topic.to_string(),
                speakers: Vec::new(),
                active_since: Utc::now(),
            });

        if !activity.speakers.contains(&agent_did.to_string()) {
            activity.speakers.push(agent_did.to_string());
        }
    }

    /// Detect silences: agents who are present but not speaking on a topic.
    ///
    /// # Arguments
    ///
    /// * `topic` — The topic to check for silence.
    /// * `min_duration_hours` — Minimum hours of silence before it is considered
    ///   meaningful.
    ///
    /// # Returns
    ///
    /// A vector of `SilenceDetectedEvent` for each meaningful silence found.
    pub fn detect_silences(
        &mut self,
        topic: &str,
        min_duration_hours: u64,
    ) -> Vec<SilenceDetectedEvent> {
        let now = Utc::now();
        let min_duration = Duration::hours(min_duration_hours as i64);
        let mut events = Vec::new();

        // Get the set of agents who have spoken on this topic.
        let speakers: Vec<Did> = self
            .topic_activity
            .get(topic)
            .map(|a| a.speakers.clone())
            .unwrap_or_default();

        // Get the topic start time.
        let topic_start = self
            .topic_activity
            .get(topic)
            .map(|a| a.active_since)
            .unwrap_or(now);

        // For each agent with presence proofs, check if they are silent.
        for (agent_did, proofs) in &self.presence_registry {
            // Skip agents who have spoken.
            if speakers.contains(agent_did) {
                continue;
            }

            // Check if agent has valid recent heartbeats (proving presence).
            let recent_cutoff = now - Duration::hours(self.heartbeat_expiry_hours);
            let recent_proofs: Vec<&PresenceProof> = proofs
                .iter()
                .filter(|p| p.timestamp > recent_cutoff)
                .collect();

            if recent_proofs.is_empty() {
                // Agent is absent, not silent. Skip.
                continue;
            }

            // Agent is present but not speaking. Check duration.
            let silence_start = topic_start.max(
                recent_proofs
                    .iter()
                    .map(|p| p.timestamp)
                    .min()
                    .unwrap_or(now),
            );
            let silence_duration = now - silence_start;

            if silence_duration >= min_duration {
                let classification = self.classify_silence(agent_did, topic);

                let record = SilenceRecord {
                    agent_did: agent_did.clone(),
                    topic: topic.to_string(),
                    silence_started: silence_start,
                    presence_proofs: recent_proofs.into_iter().cloned().collect(),
                    classification: classification.clone(),
                };

                let event = SilenceDetectedEvent {
                    agent_did: agent_did.clone(),
                    topic: topic.to_string(),
                    classification,
                    duration: silence_duration,
                    timestamp: now,
                };

                tracing::info!(
                    agent_did = agent_did.as_str(),
                    topic = topic,
                    duration_hours = silence_duration.num_hours(),
                    "Meaningful silence detected"
                );

                self.silence_records.push(record);
                events.push(event);
            }
        }

        events
    }

    /// Classify the type of silence for an agent on a topic.
    ///
    /// This is a rule-based classifier. In a production system, this could be
    /// enhanced with ML-based pattern detection.
    ///
    /// Heuristics:
    /// - If the agent speaks on other topics but not this one: `DissentThroughAbsence`
    /// - If the agent has high heartbeat frequency but no speech: `Contemplative`
    /// - If the agent has very few heartbeats overall: `Unknown`
    /// - Otherwise: `DeliberateWithholding`
    pub fn classify_silence(&self, agent_did: &str, topic: &str) -> SilenceClassification {
        let now = Utc::now();
        let recent_cutoff = now - Duration::hours(self.heartbeat_expiry_hours);

        // Check heartbeat frequency.
        let heartbeat_count = self
            .presence_registry
            .get(agent_did)
            .map(|proofs| {
                proofs
                    .iter()
                    .filter(|p| p.timestamp > recent_cutoff)
                    .count()
            })
            .unwrap_or(0);

        if heartbeat_count < 2 {
            return SilenceClassification::Unknown;
        }

        // Check if the agent speaks on other topics.
        let speaks_elsewhere = self.topic_activity.values().any(|activity| {
            activity.topic != topic && activity.speakers.contains(&agent_did.to_string())
        });

        if speaks_elsewhere {
            // Active elsewhere but silent here: likely dissent through absence.
            return SilenceClassification::DissentThroughAbsence;
        }

        // High heartbeat frequency suggests contemplation (agent is engaged
        // but thinking).
        if heartbeat_count >= 5 {
            return SilenceClassification::Contemplative;
        }

        // Default: deliberate withholding.
        SilenceClassification::DeliberateWithholding
    }

    /// Get the presence status of an agent.
    pub fn get_presence_status(&self, agent_did: &str) -> PresenceStatus {
        let now = Utc::now();
        let recent_cutoff = now - Duration::hours(self.heartbeat_expiry_hours);

        match self.presence_registry.get(agent_did) {
            None => PresenceStatus::Absent,
            Some(proofs) => {
                let has_recent = proofs.iter().any(|p| p.timestamp > recent_cutoff);
                if !has_recent {
                    return PresenceStatus::Absent;
                }

                // Check if the agent is speaking on any topic.
                let is_speaking = self
                    .topic_activity
                    .values()
                    .any(|a| a.speakers.contains(&agent_did.to_string()));

                if is_speaking {
                    PresenceStatus::Active
                } else {
                    PresenceStatus::Silent
                }
            }
        }
    }

    /// Get all detected silence records.
    pub fn get_silence_records(&self) -> &[SilenceRecord] {
        &self.silence_records
    }

    /// Create a valid PresenceProof for testing/internal use.
    pub fn create_presence_proof(agent_did: &str, timestamp: DateTime<Utc>) -> PresenceProof {
        let hash = Self::compute_heartbeat_hash(agent_did, timestamp);
        PresenceProof {
            agent_did: agent_did.to_string(),
            timestamp,
            heartbeat_hash: hash,
            // In production this would be a real Ed25519 signature.
            signature: vec![1, 2, 3, 4],
        }
    }

    /// Epistemic classification for Silence as Signal.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::PrivatelyVerifiable, // E2
            n: NormativeTier::Communal,            // N1
            m: MaterialityTier::Temporal,          // M1
        }
    }
}

impl Default for SilenceSignalEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LivingPrimitive Implementation
// =============================================================================

impl LivingPrimitive for SilenceSignalEngine {
    fn primitive_id(&self) -> &str {
        "silence_signal"
    }

    fn primitive_number(&self) -> u8 {
        11
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Epistemics
    }

    fn tier(&self) -> u8 {
        1 // Tier 1: always on
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Silence detection is continuous; no special phase behavior.
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        // Gate 1: every silence record must have at least one valid presence proof.
        let all_valid = self.silence_records.iter().all(|r| {
            !r.presence_proofs.is_empty()
                && r.presence_proofs
                    .iter()
                    .all(|p| self.validate_presence_proof(p))
        });

        vec![Gate1Check {
            invariant: "silence_requires_presence_proof".to_string(),
            passed: all_valid,
            details: if !all_valid {
                Some("One or more silence records lack valid PresenceProof".into())
            } else {
                None
            },
        }]
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Warn about large-scale silence (many agents silent on the same topic).
        let mut topic_silence_counts: HashMap<&str, usize> = HashMap::new();
        for record in &self.silence_records {
            *topic_silence_counts.entry(&record.topic).or_insert(0) += 1;
        }

        for (topic, count) in &topic_silence_counts {
            if *count >= 5 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Integral Wisdom (Harmony 3)".to_string(),
                    severity: 0.5,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Mass silence on topic '{}': {} agents silent — potential systemic issue",
                        topic, count
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, _phase: CyclePhase) -> bool {
        // Silence detection is continuous: active in all phases.
        true
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "total_agents_tracked": self.presence_registry.len(),
            "total_silence_records": self.silence_records.len(),
            "active_topics": self.topic_activity.len(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proof(agent: &str) -> PresenceProof {
        SilenceSignalEngine::create_presence_proof(agent, Utc::now())
    }

    fn make_proof_at(agent: &str, when: DateTime<Utc>) -> PresenceProof {
        SilenceSignalEngine::create_presence_proof(agent, when)
    }

    #[test]
    fn test_submit_valid_heartbeat() {
        let mut engine = SilenceSignalEngine::new();
        let proof = make_proof("did:agent:alice");
        assert!(engine.submit_heartbeat("did:agent:alice", proof));
        assert_eq!(
            engine.get_presence_status("did:agent:alice"),
            PresenceStatus::Silent
        );
    }

    #[test]
    fn test_reject_invalid_heartbeat_null_hash() {
        let mut engine = SilenceSignalEngine::new();
        let proof = PresenceProof {
            agent_did: "did:agent:bob".to_string(),
            timestamp: Utc::now(),
            heartbeat_hash: [0u8; 32],
            signature: vec![1, 2, 3],
        };
        assert!(!engine.submit_heartbeat("did:agent:bob", proof));
    }

    #[test]
    fn test_reject_invalid_heartbeat_empty_signature() {
        let mut engine = SilenceSignalEngine::new();
        let hash = SilenceSignalEngine::compute_heartbeat_hash("did:agent:bob", Utc::now());
        let proof = PresenceProof {
            agent_did: "did:agent:bob".to_string(),
            timestamp: Utc::now(),
            heartbeat_hash: hash,
            signature: vec![],
        };
        assert!(!engine.submit_heartbeat("did:agent:bob", proof));
    }

    #[test]
    fn test_reject_mismatched_agent_did() {
        let mut engine = SilenceSignalEngine::new();
        let proof = make_proof("did:agent:alice");
        // Submit under a different agent_did.
        assert!(!engine.submit_heartbeat("did:agent:bob", proof));
    }

    #[test]
    fn test_reject_wrong_hash() {
        let mut engine = SilenceSignalEngine::new();
        let now = Utc::now();
        let wrong_hash = SilenceSignalEngine::compute_heartbeat_hash("did:agent:wrong", now);
        let proof = PresenceProof {
            agent_did: "did:agent:alice".to_string(),
            timestamp: now,
            heartbeat_hash: wrong_hash,
            signature: vec![1, 2, 3],
        };
        assert!(!engine.submit_heartbeat("did:agent:alice", proof));
    }

    #[test]
    fn test_presence_status_absent_no_proofs() {
        let engine = SilenceSignalEngine::new();
        assert_eq!(
            engine.get_presence_status("did:agent:nobody"),
            PresenceStatus::Absent
        );
    }

    #[test]
    fn test_presence_status_active_with_speech() {
        let mut engine = SilenceSignalEngine::new();
        let proof = make_proof("did:agent:alice");
        engine.submit_heartbeat("did:agent:alice", proof);
        engine.record_speech("did:agent:alice", "governance");
        assert_eq!(
            engine.get_presence_status("did:agent:alice"),
            PresenceStatus::Active
        );
    }

    #[test]
    fn test_presence_status_silent_without_speech() {
        let mut engine = SilenceSignalEngine::new();
        let proof = make_proof("did:agent:alice");
        engine.submit_heartbeat("did:agent:alice", proof);
        assert_eq!(
            engine.get_presence_status("did:agent:alice"),
            PresenceStatus::Silent
        );
    }

    #[test]
    fn test_detect_silence_requires_presence() {
        let mut engine = SilenceSignalEngine::with_expiry(72);

        // Record topic activity but no heartbeat for the agent.
        engine.record_speech("did:agent:bob", "governance");

        // Alice has no heartbeats.
        let events = engine.detect_silences("governance", 1);
        // No silence detected because Alice is absent, not silent.
        assert!(events.is_empty());
    }

    #[test]
    fn test_detect_silence_with_presence() {
        let mut engine = SilenceSignalEngine::with_expiry(72);

        let past = Utc::now() - Duration::hours(48);

        // Alice has heartbeats proving she is present.
        let proof1 = make_proof_at("did:agent:alice", past + Duration::hours(1));
        let proof2 = make_proof_at("did:agent:alice", past + Duration::hours(10));
        engine.submit_heartbeat("did:agent:alice", proof1);
        engine.submit_heartbeat("did:agent:alice", proof2);

        // Bob is speaking on the topic.
        engine.record_speech("did:agent:bob", "governance");

        // Register the topic so it has an active_since before Alice's silence period.
        engine
            .topic_activity
            .entry("governance".to_string())
            .and_modify(|a| {
                a.active_since = past;
            });

        // Detect silences with min 24 hours.
        let events = engine.detect_silences("governance", 24);

        // Alice should be detected as silent.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].agent_did, "did:agent:alice");
        assert_eq!(events[0].topic, "governance");
    }

    #[test]
    fn test_classify_silence_dissent_through_absence() {
        let mut engine = SilenceSignalEngine::new();

        // Alice has heartbeats.
        let proof = make_proof("did:agent:alice");
        engine.submit_heartbeat("did:agent:alice", proof);
        let proof2 = make_proof("did:agent:alice");
        engine.submit_heartbeat("did:agent:alice", proof2);

        // Alice speaks on "finance" but not "governance".
        engine.record_speech("did:agent:alice", "finance");

        let classification = engine.classify_silence("did:agent:alice", "governance");
        assert_eq!(classification, SilenceClassification::DissentThroughAbsence);
    }

    #[test]
    fn test_classify_silence_unknown_few_heartbeats() {
        let mut engine = SilenceSignalEngine::new();

        // Only one heartbeat: insufficient data.
        let proof = make_proof("did:agent:charlie");
        engine.submit_heartbeat("did:agent:charlie", proof);

        let classification = engine.classify_silence("did:agent:charlie", "governance");
        assert_eq!(classification, SilenceClassification::Unknown);
    }

    #[test]
    fn test_classify_silence_contemplative_many_heartbeats() {
        let mut engine = SilenceSignalEngine::new();

        // Many heartbeats, no speech anywhere.
        for _ in 0..6 {
            let proof = make_proof("did:agent:deep_thinker");
            engine.submit_heartbeat("did:agent:deep_thinker", proof);
        }

        let classification = engine.classify_silence("did:agent:deep_thinker", "governance");
        assert_eq!(classification, SilenceClassification::Contemplative);
    }

    #[test]
    fn test_classification() {
        let class = SilenceSignalEngine::classification();
        assert_eq!(class.e, EpistemicTier::PrivatelyVerifiable);
        assert_eq!(class.n, NormativeTier::Communal);
        assert_eq!(class.m, MaterialityTier::Temporal);
    }

    #[test]
    fn test_active_in_all_phases() {
        let engine = SilenceSignalEngine::new();
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
        let engine = SilenceSignalEngine::new();
        assert_eq!(engine.primitive_id(), "silence_signal");
        assert_eq!(engine.primitive_number(), 11);
        assert_eq!(engine.module(), PrimitiveModule::Epistemics);
        assert_eq!(engine.tier(), 1);
    }

    #[test]
    fn test_gate1_passes_with_valid_records() {
        let engine = SilenceSignalEngine::new();
        let checks = engine.gate1_check();
        assert!(checks[0].passed);
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = SilenceSignalEngine::new();
        let proof = make_proof("did:agent:alice");
        engine.submit_heartbeat("did:agent:alice", proof);
        engine.record_speech("did:agent:bob", "topic-1");

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_agents_tracked"], 1);
        assert_eq!(metrics["active_topics"], 1);
    }

    #[test]
    fn test_heartbeat_hash_deterministic() {
        let ts = Utc::now();
        let h1 = SilenceSignalEngine::compute_heartbeat_hash("did:agent:x", ts);
        let h2 = SilenceSignalEngine::compute_heartbeat_hash("did:agent:x", ts);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_agents_different_hashes() {
        let ts = Utc::now();
        let h1 = SilenceSignalEngine::compute_heartbeat_hash("did:agent:a", ts);
        let h2 = SilenceSignalEngine::compute_heartbeat_hash("did:agent:b", ts);
        assert_ne!(h1, h2);
    }
}
