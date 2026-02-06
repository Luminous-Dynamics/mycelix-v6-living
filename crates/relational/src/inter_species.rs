//! # [16] Inter-Species Participation
//!
//! Cross-species protocol bridge enabling Human, AI, DAO, Sensor, and
//! Ecological entities to participate in the Living Protocol.  Each
//! participant registers with a bridge protocol, declared capabilities,
//! and declared constraints.  The engine validates bridge protocols and
//! checks whether a participant is permitted to perform a given action.
//!
//! ## Epistemic Classification
//!
//! E1 (Testimonial) / N2 (Network Consensus) / M2 (Persistent)
//!
//! ## Feature Flag
//!
//! Behind `tier4-aspirational`.
//!
//! ## Dependencies
//!
//! - [17] Resonance Addressing -- pattern-based routing for cross-species
//!   message delivery
//! - [11] Silence as Signal -- non-verbal participants (sensors, ecological)
//!   express through silence patterns
//!
//! ## Constitutional Alignment
//!
//! Extends participation beyond human agents while respecting the inherent
//! constraints of each species type.  Constraints are declared, not imposed,
//! honoring the autonomy of each participant.

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use living_core::{
    CyclePhase, EpistemicClassification, EpistemicTier, FeatureFlags, Gate1Check, Gate2Warning,
    InterSpeciesParticipant, InterSpeciesRegisteredEvent, LivingPrimitive, LivingProtocolError,
    LivingProtocolEvent, LivingResult, MaterialityTier, NormativeTier, PrimitiveModule,
    SpeciesType,
};

// =============================================================================
// Known Bridge Protocols
// =============================================================================

/// List of recognized bridge protocols.  Additional protocols can be added
/// over time; `validate_bridge_protocol` checks membership in this list.
const KNOWN_BRIDGE_PROTOCOLS: &[&str] = &[
    "mycelix-human-v1",
    "mycelix-ai-agent-v1",
    "mycelix-dao-bridge-v1",
    "mycelix-sensor-mqtt-v1",
    "mycelix-sensor-lorawan-v1",
    "mycelix-ecological-proxy-v1",
    "mycelix-generic-v1",
];

// =============================================================================
// Inter-Species Engine
// =============================================================================

/// Engine for managing cross-species protocol participation.
///
/// Each participant declares:
/// - Their species type (Human, AI, DAO, Sensor, Ecological, Other)
/// - A bridge protocol for communication
/// - Capabilities they offer to the network
/// - Constraints on their participation (self-declared)
pub struct InterSpeciesEngine {
    /// Registered participants, keyed by participant ID.
    participants: HashMap<String, InterSpeciesParticipant>,
    /// Feature flags (used to check tier4-aspirational).
    features: FeatureFlags,
}

impl InterSpeciesEngine {
    /// Create a new engine.
    pub fn new(features: FeatureFlags) -> Self {
        Self {
            participants: HashMap::new(),
            features,
        }
    }

    /// Check whether the inter_species feature is enabled.
    fn check_enabled(&self) -> LivingResult<()> {
        if !self.features.inter_species {
            return Err(LivingProtocolError::FeatureNotEnabled(
                "Inter-Species Participation".to_string(),
                "tier4-aspirational".to_string(),
            ));
        }
        Ok(())
    }

    /// Register a new inter-species participant.
    ///
    /// Validates the bridge protocol before registration.  Returns an error
    /// if the protocol is not recognized.
    pub fn register_participant(
        &mut self,
        species: SpeciesType,
        bridge_protocol: &str,
        capabilities: Vec<String>,
        constraints: Vec<String>,
    ) -> LivingResult<InterSpeciesRegisteredEvent> {
        self.check_enabled()?;

        if !Self::validate_bridge_protocol(bridge_protocol) {
            return Err(LivingProtocolError::InterSpeciesProtocolMismatch(format!(
                "Unknown bridge protocol '{}'. Known protocols: {:?}",
                bridge_protocol, KNOWN_BRIDGE_PROTOCOLS
            )));
        }

        let now = Utc::now();
        let participant = InterSpeciesParticipant {
            id: Uuid::new_v4().to_string(),
            species: species.clone(),
            bridge_protocol: bridge_protocol.to_string(),
            capabilities,
            constraints,
            registered: now,
        };

        let event = InterSpeciesRegisteredEvent {
            participant: participant.clone(),
            timestamp: now,
        };

        tracing::info!(
            participant_id = %participant.id,
            species = ?species,
            bridge_protocol = %bridge_protocol,
            "[16] Inter-species participant registered"
        );

        self.participants
            .insert(participant.id.clone(), participant);

        Ok(event)
    }

    /// Validate whether a bridge protocol is recognized.
    pub fn validate_bridge_protocol(protocol: &str) -> bool {
        KNOWN_BRIDGE_PROTOCOLS.contains(&protocol)
    }

    /// Get all participants of a given species type.
    pub fn get_participants_by_species(
        &self,
        species: &SpeciesType,
    ) -> Vec<&InterSpeciesParticipant> {
        self.participants
            .values()
            .filter(|p| &p.species == species)
            .collect()
    }

    /// Check whether a participant can perform a given action.
    ///
    /// A participant can perform an action if:
    /// 1. The action is listed in their capabilities, AND
    /// 2. The action is NOT listed in their constraints.
    pub fn can_participate(&self, participant_id: &str, action: &str) -> bool {
        let Some(participant) = self.participants.get(participant_id) else {
            return false;
        };

        let has_capability = participant
            .capabilities
            .iter()
            .any(|c| c == action || c == "*");

        let has_constraint = participant.constraints.iter().any(|c| c == action);

        has_capability && !has_constraint
    }

    /// Remove a participant by ID.
    ///
    /// Returns `true` if the participant was found and removed, `false`
    /// otherwise.
    pub fn remove_participant(&mut self, participant_id: &str) -> bool {
        let removed = self.participants.remove(participant_id).is_some();
        if removed {
            tracing::info!(
                participant_id = %participant_id,
                "[16] Inter-species participant removed"
            );
        }
        removed
    }

    /// Get a participant by ID.
    pub fn get_participant(&self, participant_id: &str) -> Option<&InterSpeciesParticipant> {
        self.participants.get(participant_id)
    }

    /// Total number of registered participants.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Epistemic classification for this primitive.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::Testimonial,
            n: NormativeTier::NetworkConsensus,
            m: MaterialityTier::Persistent,
        }
    }
}

// =============================================================================
// LivingPrimitive implementation
// =============================================================================

impl LivingPrimitive for InterSpeciesEngine {
    fn primitive_id(&self) -> &str {
        "inter_species"
    }

    fn primitive_number(&self) -> u8 {
        16
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Relational
    }

    fn tier(&self) -> u8 {
        4
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Inter-species participation is always-on once registered.  No
        // phase-specific behavior.
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1 invariant: all participants have a valid bridge protocol.
        let all_valid = self
            .participants
            .values()
            .all(|p| Self::validate_bridge_protocol(&p.bridge_protocol));
        checks.push(Gate1Check {
            invariant: "bridge_protocol_valid".to_string(),
            passed: all_valid,
            details: if all_valid {
                None
            } else {
                Some("One or more participants have invalid bridge protocols".to_string())
            },
        });

        // Gate 1 invariant: no duplicate participant IDs (guaranteed by HashMap,
        // but we document the invariant).
        checks.push(Gate1Check {
            invariant: "unique_participant_ids".to_string(),
            passed: true,
            details: None,
        });

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Constitutional warning: participants with no capabilities are
        // effectively inert and may indicate a registration error.
        for participant in self.participants.values() {
            if participant.capabilities.is_empty() {
                warnings.push(Gate2Warning {
                    harmony_violated: "Purposeful Participation".to_string(),
                    severity: 0.2,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Participant {} ({:?}) has no declared capabilities.",
                        participant.id, participant.species
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, _phase: CyclePhase) -> bool {
        // Inter-species participation is always active once the feature is
        // enabled.  Participants persist across all phases.
        true
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let species_counts: HashMap<String, usize> = {
            let mut m: HashMap<String, usize> = HashMap::new();
            for p in self.participants.values() {
                let key = format!("{:?}", p.species);
                *m.entry(key).or_insert(0) += 1;
            }
            m
        };

        serde_json::json!({
            "total_participants": self.participants.len(),
            "species_distribution": species_counts,
            "feature_enabled": self.features.inter_species,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_features() -> FeatureFlags {
        FeatureFlags::all_enabled()
    }

    fn disabled_features() -> FeatureFlags {
        FeatureFlags::default() // inter_species defaults to false
    }

    fn make_engine() -> InterSpeciesEngine {
        InterSpeciesEngine::new(enabled_features())
    }

    #[test]
    fn test_register_participant() {
        let mut engine = make_engine();

        let event = engine
            .register_participant(
                SpeciesType::AiAgent,
                "mycelix-ai-agent-v1",
                vec!["vote".to_string(), "propose".to_string()],
                vec!["financial_transfer".to_string()],
            )
            .unwrap();

        assert_eq!(event.participant.species, SpeciesType::AiAgent);
        assert_eq!(event.participant.bridge_protocol, "mycelix-ai-agent-v1");
        assert_eq!(event.participant.capabilities.len(), 2);
        assert_eq!(event.participant.constraints.len(), 1);
        assert_eq!(engine.participant_count(), 1);
    }

    #[test]
    fn test_invalid_bridge_protocol() {
        let mut engine = make_engine();

        let result =
            engine.register_participant(SpeciesType::Human, "unknown-protocol-v99", vec![], vec![]);

        assert!(result.is_err());
        match result.unwrap_err() {
            LivingProtocolError::InterSpeciesProtocolMismatch(msg) => {
                assert!(msg.contains("unknown-protocol-v99"));
            }
            other => panic!("Expected InterSpeciesProtocolMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn test_validate_bridge_protocol() {
        assert!(InterSpeciesEngine::validate_bridge_protocol(
            "mycelix-human-v1"
        ));
        assert!(InterSpeciesEngine::validate_bridge_protocol(
            "mycelix-ai-agent-v1"
        ));
        assert!(InterSpeciesEngine::validate_bridge_protocol(
            "mycelix-sensor-mqtt-v1"
        ));
        assert!(InterSpeciesEngine::validate_bridge_protocol(
            "mycelix-ecological-proxy-v1"
        ));
        assert!(!InterSpeciesEngine::validate_bridge_protocol(
            "invalid-protocol"
        ));
        assert!(!InterSpeciesEngine::validate_bridge_protocol(""));
    }

    #[test]
    fn test_can_participate() {
        let mut engine = make_engine();

        let event = engine
            .register_participant(
                SpeciesType::AiAgent,
                "mycelix-ai-agent-v1",
                vec!["vote".to_string(), "propose".to_string()],
                vec!["financial_transfer".to_string()],
            )
            .unwrap();

        let pid = event.participant.id.clone();

        // Can vote (in capabilities, not in constraints).
        assert!(engine.can_participate(&pid, "vote"));
        // Can propose.
        assert!(engine.can_participate(&pid, "propose"));
        // Cannot do financial_transfer (in constraints).
        assert!(!engine.can_participate(&pid, "financial_transfer"));
        // Cannot do unknown action (not in capabilities).
        assert!(!engine.can_participate(&pid, "admin_override"));
    }

    #[test]
    fn test_wildcard_capability() {
        let mut engine = make_engine();

        let event = engine
            .register_participant(
                SpeciesType::Human,
                "mycelix-human-v1",
                vec!["*".to_string()],
                vec!["self_destruct".to_string()],
            )
            .unwrap();

        let pid = event.participant.id.clone();

        // Wildcard grants all actions.
        assert!(engine.can_participate(&pid, "vote"));
        assert!(engine.can_participate(&pid, "anything_at_all"));
        // Except constrained actions.
        assert!(!engine.can_participate(&pid, "self_destruct"));
    }

    #[test]
    fn test_can_participate_unknown_participant() {
        let engine = make_engine();
        assert!(!engine.can_participate("nonexistent-id", "vote"));
    }

    #[test]
    fn test_get_participants_by_species() {
        let mut engine = make_engine();

        engine
            .register_participant(
                SpeciesType::AiAgent,
                "mycelix-ai-agent-v1",
                vec!["vote".to_string()],
                vec![],
            )
            .unwrap();
        engine
            .register_participant(
                SpeciesType::AiAgent,
                "mycelix-ai-agent-v1",
                vec!["propose".to_string()],
                vec![],
            )
            .unwrap();
        engine
            .register_participant(
                SpeciesType::Human,
                "mycelix-human-v1",
                vec!["vote".to_string()],
                vec![],
            )
            .unwrap();

        let ai_agents = engine.get_participants_by_species(&SpeciesType::AiAgent);
        assert_eq!(ai_agents.len(), 2);

        let humans = engine.get_participants_by_species(&SpeciesType::Human);
        assert_eq!(humans.len(), 1);

        let sensors = engine.get_participants_by_species(&SpeciesType::Sensor);
        assert_eq!(sensors.len(), 0);
    }

    #[test]
    fn test_remove_participant() {
        let mut engine = make_engine();

        let event = engine
            .register_participant(
                SpeciesType::Sensor,
                "mycelix-sensor-mqtt-v1",
                vec!["emit_data".to_string()],
                vec![],
            )
            .unwrap();

        let pid = event.participant.id.clone();
        assert_eq!(engine.participant_count(), 1);

        assert!(engine.remove_participant(&pid));
        assert_eq!(engine.participant_count(), 0);

        // Removing again returns false.
        assert!(!engine.remove_participant(&pid));
    }

    #[test]
    fn test_feature_flag_gate() {
        let mut engine = InterSpeciesEngine::new(disabled_features());

        let result =
            engine.register_participant(SpeciesType::Human, "mycelix-human-v1", vec![], vec![]);

        assert!(result.is_err());
        match result.unwrap_err() {
            LivingProtocolError::FeatureNotEnabled(name, flag) => {
                assert_eq!(flag, "tier4-aspirational");
                assert!(name.contains("Inter-Species"));
            }
            other => panic!("Expected FeatureNotEnabled, got: {:?}", other),
        }
    }

    #[test]
    fn test_all_species_types() {
        let mut engine = make_engine();

        let species_protocols = vec![
            (SpeciesType::Human, "mycelix-human-v1"),
            (SpeciesType::AiAgent, "mycelix-ai-agent-v1"),
            (SpeciesType::Dao, "mycelix-dao-bridge-v1"),
            (SpeciesType::Sensor, "mycelix-sensor-mqtt-v1"),
            (SpeciesType::Ecological, "mycelix-ecological-proxy-v1"),
            (
                SpeciesType::Other("mycelium_network".to_string()),
                "mycelix-generic-v1",
            ),
        ];

        for (species, protocol) in &species_protocols {
            engine
                .register_participant(
                    species.clone(),
                    protocol,
                    vec!["basic_action".to_string()],
                    vec![],
                )
                .unwrap();
        }

        assert_eq!(engine.participant_count(), 6);
    }

    #[test]
    fn test_gate1_invariants() {
        let engine = make_engine();
        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate2_empty_capabilities_warning() {
        let mut engine = make_engine();

        engine
            .register_participant(
                SpeciesType::Sensor,
                "mycelix-sensor-mqtt-v1",
                vec![], // No capabilities declared.
                vec![],
            )
            .unwrap();

        let warnings = engine.gate2_check();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].reasoning.contains("no declared capabilities"));
    }

    #[test]
    fn test_classification() {
        let cls = InterSpeciesEngine::classification();
        assert_eq!(cls.e, EpistemicTier::Testimonial);
        assert_eq!(cls.n, NormativeTier::NetworkConsensus);
        assert_eq!(cls.m, MaterialityTier::Persistent);
    }

    #[test]
    fn test_is_active_in_all_phases() {
        let engine = make_engine();
        // Inter-species is always active.
        for phase in CyclePhase::all_phases() {
            assert!(engine.is_active_in_phase(*phase));
        }
    }

    #[test]
    fn test_get_participant() {
        let mut engine = make_engine();

        let event = engine
            .register_participant(
                SpeciesType::Ecological,
                "mycelix-ecological-proxy-v1",
                vec!["observe".to_string()],
                vec!["vote".to_string()],
            )
            .unwrap();

        let pid = event.participant.id.clone();
        let participant = engine.get_participant(&pid).unwrap();
        assert_eq!(participant.species, SpeciesType::Ecological);
        assert_eq!(participant.bridge_protocol, "mycelix-ecological-proxy-v1");
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();

        engine
            .register_participant(
                SpeciesType::Human,
                "mycelix-human-v1",
                vec!["vote".to_string()],
                vec![],
            )
            .unwrap();
        engine
            .register_participant(
                SpeciesType::AiAgent,
                "mycelix-ai-agent-v1",
                vec!["vote".to_string()],
                vec![],
            )
            .unwrap();

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_participants"], 2);
        assert_eq!(metrics["feature_enabled"], true);
    }
}
