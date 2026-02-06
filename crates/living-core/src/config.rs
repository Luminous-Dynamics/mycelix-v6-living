//! Configuration for the Living Protocol Layer.

use serde::{Deserialize, Serialize};

/// Master configuration for the Living Protocol Layer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LivingProtocolConfig {
    /// Metabolism cycle configuration
    pub cycle: CycleConfig,
    /// Wound healing configuration
    pub wound_healing: WoundHealingConfig,
    /// Kenosis configuration
    pub kenosis: KenosisConfig,
    /// Metabolic trust configuration
    pub metabolic_trust: MetabolicTrustConfig,
    /// Composting configuration
    pub composting: CompostingConfig,
    /// Silence detection configuration
    pub silence: SilenceConfig,
    /// Beauty scoring configuration
    pub beauty: BeautyConfig,
    /// Entanglement configuration
    pub entanglement: EntanglementConfig,
    /// Shadow integration configuration
    pub shadow: ShadowConfig,
    /// Negative capability configuration
    pub negative_capability: NegativeCapabilityConfig,
    /// Dreaming configuration
    pub dreaming: DreamingConfig,
    /// Feature flags
    pub features: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleConfig {
    /// Total cycle length in days (default: 28, lunar cycle)
    pub cycle_days: u32,
    /// Whether to use real time or simulated time
    pub simulated_time: bool,
    /// Time acceleration factor for testing (1.0 = real time)
    pub time_acceleration: f64,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            cycle_days: 28,
            simulated_time: false,
            time_acceleration: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoundHealingConfig {
    /// Minimum time in hemostasis phase (hours)
    pub min_hemostasis_hours: u64,
    /// Maximum time in hemostasis phase (hours)
    pub max_hemostasis_hours: u64,
    /// Restitution deadline multiplier based on severity
    pub restitution_deadline_days: u32,
    /// Scar tissue strength multiplier range
    pub scar_strength_min: f64,
    pub scar_strength_max: f64,
}

impl Default for WoundHealingConfig {
    fn default() -> Self {
        Self {
            min_hemostasis_hours: 1,
            max_hemostasis_hours: 72,
            restitution_deadline_days: 28,
            scar_strength_min: 1.1,
            scar_strength_max: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KenosisConfig {
    /// Maximum reputation percentage that can be released per cycle
    pub max_release_per_cycle: f64,
    /// Whether kenosis is irrevocable (should always be true in production)
    pub irrevocable: bool,
}

impl Default for KenosisConfig {
    fn default() -> Self {
        Self {
            max_release_per_cycle: 0.20,
            irrevocable: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicTrustConfig {
    /// Weight of MATL composite in metabolic trust
    pub matl_weight: f64,
    /// Weight of throughput
    pub throughput_weight: f64,
    /// Weight of resilience
    pub resilience_weight: f64,
    /// Weight of composting contribution
    pub composting_weight: f64,
    /// Update interval in seconds
    pub update_interval_secs: u64,
}

impl Default for MetabolicTrustConfig {
    fn default() -> Self {
        Self {
            matl_weight: 0.35,
            throughput_weight: 0.25,
            resilience_weight: 0.20,
            composting_weight: 0.20,
            update_interval_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompostingConfig {
    /// Minimum decomposition time in days
    pub min_decomposition_days: u32,
    /// Maximum nutrients per entity
    pub max_nutrients_per_entity: usize,
    /// Publish nutrients to DKG automatically
    pub auto_publish_nutrients: bool,
}

impl Default for CompostingConfig {
    fn default() -> Self {
        Self {
            min_decomposition_days: 3,
            max_nutrients_per_entity: 10,
            auto_publish_nutrients: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceConfig {
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Minimum silence duration before classification (hours)
    pub min_silence_hours: u64,
    /// Maximum missed heartbeats before presence proof expires
    pub max_missed_heartbeats: u32,
}

impl Default for SilenceConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: 300,
            min_silence_hours: 24,
            max_missed_heartbeats: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeautyConfig {
    /// Weight for each beauty component
    pub symmetry_weight: f64,
    pub economy_weight: f64,
    pub resonance_weight: f64,
    pub surprise_weight: f64,
    pub completeness_weight: f64,
    /// Minimum beauty score for a proposal to be considered valid
    pub minimum_beauty_threshold: f64,
}

impl Default for BeautyConfig {
    fn default() -> Self {
        Self {
            symmetry_weight: 0.20,
            economy_weight: 0.20,
            resonance_weight: 0.25,
            surprise_weight: 0.15,
            completeness_weight: 0.20,
            minimum_beauty_threshold: 0.4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementConfig {
    /// Minimum co-creation events to form entanglement
    pub min_co_creation_events: u32,
    /// Base entanglement strength
    pub base_strength: f64,
    /// Decay rate per day without interaction
    pub decay_rate_per_day: f64,
    /// Minimum strength before entanglement is broken
    pub min_strength_threshold: f64,
}

impl Default for EntanglementConfig {
    fn default() -> Self {
        Self {
            min_co_creation_events: 3,
            base_strength: 0.5,
            decay_rate_per_day: 0.02,
            min_strength_threshold: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    /// How often to run shadow surfacing (in cycles)
    pub surfacing_interval_cycles: u32,
    /// Maximum items to surface per shadow phase
    pub max_surface_per_phase: usize,
    /// Spectral K anomaly threshold for triggering
    pub spectral_k_anomaly_threshold: f64,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            surfacing_interval_cycles: 1,
            max_surface_per_phase: 10,
            spectral_k_anomaly_threshold: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeCapabilityConfig {
    /// Minimum hold duration (days)
    pub min_hold_days: u32,
    /// Maximum hold duration (days) before auto-release
    pub max_hold_days: u32,
    /// Whether voting is blocked on held claims
    pub block_voting: bool,
}

impl Default for NegativeCapabilityConfig {
    fn default() -> Self {
        Self {
            min_hold_days: 7,
            max_hold_days: 90,
            block_voting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamingConfig {
    /// Confirmation threshold for dream proposals
    pub confirmation_threshold: f64,
    /// Whether financial operations are blocked during dreams
    pub block_financial_in_dreams: bool,
    /// Maximum dream proposals per cycle
    pub max_proposals_per_cycle: usize,
}

impl Default for DreamingConfig {
    fn default() -> Self {
        Self {
            confirmation_threshold: 0.67,
            block_financial_in_dreams: true,
            max_proposals_per_cycle: 50,
        }
    }
}

/// Feature flags controlling which primitives are active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    // Tier 1 (always on)
    pub metabolic_trust: bool,     // [3]
    pub temporal_k_vector: bool,   // [5]
    pub negative_capability: bool, // [10]
    pub silence_as_signal: bool,   // [11]
    pub beauty_as_validity: bool,  // [12]
    pub liminality: bool,          // [15]

    // Tier 2 (default on)
    pub composting: bool,           // [1]
    pub wound_healing: bool,        // [2]
    pub kenosis: bool,              // [4]
    pub shadow_integration: bool,   // [9]
    pub entangled_pairs: bool,      // [13]
    pub resonance_addressing: bool, // [17]
    pub fractal_governance: bool,   // [18]
    pub morphogenetic_fields: bool, // [19]

    // Tier 3 (experimental)
    pub field_interference: bool,   // [6]
    pub collective_dreaming: bool,  // [7]
    pub eros_attractor: bool,       // [14]
    pub time_crystal: bool,         // [20]
    pub mycelial_computation: bool, // [21]

    // Tier 4 (aspirational)
    pub emergent_personhood: bool, // [8]
    pub inter_species: bool,       // [16]
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            // Tier 1
            metabolic_trust: true,
            temporal_k_vector: true,
            negative_capability: true,
            silence_as_signal: true,
            beauty_as_validity: true,
            liminality: true,
            // Tier 2
            composting: true,
            wound_healing: true,
            kenosis: true,
            shadow_integration: true,
            entangled_pairs: true,
            resonance_addressing: true,
            fractal_governance: true,
            morphogenetic_fields: true,
            // Tier 3
            field_interference: false,
            collective_dreaming: false,
            eros_attractor: false,
            time_crystal: false,
            mycelial_computation: false,
            // Tier 4
            emergent_personhood: false,
            inter_species: false,
        }
    }
}

impl FeatureFlags {
    /// All features enabled (for testing).
    pub fn all_enabled() -> Self {
        Self {
            metabolic_trust: true,
            temporal_k_vector: true,
            negative_capability: true,
            silence_as_signal: true,
            beauty_as_validity: true,
            liminality: true,
            composting: true,
            wound_healing: true,
            kenosis: true,
            shadow_integration: true,
            entangled_pairs: true,
            resonance_addressing: true,
            fractal_governance: true,
            morphogenetic_fields: true,
            field_interference: true,
            collective_dreaming: true,
            eros_attractor: true,
            time_crystal: true,
            mycelial_computation: true,
            emergent_personhood: true,
            inter_species: true,
        }
    }

    /// Only tier 1 enabled (minimal).
    pub fn tier1_only() -> Self {
        Self {
            metabolic_trust: true,
            temporal_k_vector: true,
            negative_capability: true,
            silence_as_signal: true,
            beauty_as_validity: true,
            liminality: true,
            // Everything else off
            composting: false,
            wound_healing: false,
            kenosis: false,
            shadow_integration: false,
            entangled_pairs: false,
            resonance_addressing: false,
            fractal_governance: false,
            morphogenetic_fields: false,
            field_interference: false,
            collective_dreaming: false,
            eros_attractor: false,
            time_crystal: false,
            mycelial_computation: false,
            emergent_personhood: false,
            inter_species: false,
        }
    }
}
