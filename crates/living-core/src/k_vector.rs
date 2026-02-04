//! Extended K-Vector types for v6.0 Living Protocol Layer.
//! Adds temporal derivatives, field interference, and resonance addressing.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::types::SignatureBytes;

/// The canonical 8-dimensional K-Vector Signature.
/// Extended in v6.0 with temporal derivative tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVectorSignature {
    /// k_r: Reactivity (stimulus→response)
    pub k_r: f64,
    /// k_a: Agency (causal power)
    pub k_a: f64,
    /// k_i: Integration (coherence)
    pub k_i: f64,
    /// k_p: Prediction (anticipation)
    pub k_p: f64,
    /// k_m: Meta/Temporal (self-reflection)
    pub k_m: f64,
    /// k_s: Social (relational capacity)
    pub k_s: f64,
    /// k_h: Harmonic (normative alignment)
    pub k_h: f64,
    /// k_topo: Topological Closure (sanity check)
    pub k_topo: f64,

    pub timestamp: DateTime<Utc>,
    pub signature: SignatureBytes,
}

impl KVectorSignature {
    /// Number of dimensions.
    pub const DIMENSIONS: usize = 8;

    /// Get K-Vector as an array of f64.
    pub fn as_array(&self) -> [f64; 8] {
        [
            self.k_r, self.k_a, self.k_i, self.k_p, self.k_m, self.k_s, self.k_h, self.k_topo,
        ]
    }

    /// Create from array.
    pub fn from_array(values: [f64; 8], timestamp: DateTime<Utc>) -> Self {
        Self {
            k_r: values[0],
            k_a: values[1],
            k_i: values[2],
            k_p: values[3],
            k_m: values[4],
            k_s: values[5],
            k_h: values[6],
            k_topo: values[7],
            timestamp,
            signature: Vec::new(),
        }
    }

    /// Whether this agent passes the sanity check (K_Topo >= 0.7).
    pub fn is_sane(&self) -> bool {
        self.k_topo >= 0.7
    }

    /// Whether this agent is aligned with another (K_H distance < 0.3).
    pub fn is_aligned_with(&self, other: &KVectorSignature) -> bool {
        (self.k_h - other.k_h).abs() < 0.3
    }

    /// Combined trust decision: sane AND aligned.
    pub fn should_trust(&self, peer: &KVectorSignature) -> bool {
        peer.is_sane() && self.is_aligned_with(peer)
    }

    /// Euclidean distance to another K-Vector.
    pub fn distance(&self, other: &KVectorSignature) -> f64 {
        let a = self.as_array();
        let b = other.as_array();
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Cosine similarity with another K-Vector.
    pub fn cosine_similarity(&self, other: &KVectorSignature) -> f64 {
        let a = self.as_array();
        let b = other.as_array();

        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f64 = a.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let mag_b: f64 = b.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot / (mag_a * mag_b)
    }

    /// Magnitude of the K-Vector.
    pub fn magnitude(&self) -> f64 {
        self.as_array()
            .iter()
            .map(|x| x.powi(2))
            .sum::<f64>()
            .sqrt()
    }
}

// =============================================================================
// Temporal K-Vector [Primitive 5]
// =============================================================================

/// Temporal K-Vector: tracks derivatives (rate of change) of K-Vector over time.
/// Enables detection of rapid shifts, growth trajectories, and anomalies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalKVector {
    /// Current K-Vector snapshot
    pub current: KVectorSignature,
    /// First derivative: rate of change per day for each dimension
    pub velocity: [f64; 8],
    /// Second derivative: acceleration of change
    pub acceleration: [f64; 8],
    /// Historical K-Vector snapshots (ring buffer)
    pub history: Vec<KVectorSignature>,
    /// Maximum history length
    pub max_history: usize,
    /// Computed at
    pub computed_at: DateTime<Utc>,
}

impl TemporalKVector {
    pub fn new(initial: KVectorSignature, max_history: usize) -> Self {
        Self {
            current: initial,
            velocity: [0.0; 8],
            acceleration: [0.0; 8],
            history: Vec::with_capacity(max_history),
            max_history,
            computed_at: Utc::now(),
        }
    }

    /// Update with a new K-Vector observation.
    pub fn update(&mut self, new_kvec: KVectorSignature) {
        let now = new_kvec.timestamp;
        let dt = (now - self.current.timestamp).num_seconds() as f64;

        if dt <= 0.0 {
            return;
        }

        let dt_days = dt / 86400.0;
        let old = self.current.as_array();
        let new = new_kvec.as_array();
        let old_velocity = self.velocity;

        // Compute velocity (first derivative)
        for i in 0..8 {
            self.velocity[i] = (new[i] - old[i]) / dt_days;
        }

        // Compute acceleration (second derivative)
        for i in 0..8 {
            self.acceleration[i] = (self.velocity[i] - old_velocity[i]) / dt_days;
        }

        // Push old to history
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.current.clone());

        self.current = new_kvec;
        self.computed_at = now;
    }

    /// Overall rate of change (magnitude of velocity vector).
    pub fn rate_of_change(&self) -> f64 {
        self.velocity.iter().map(|v| v.powi(2)).sum::<f64>().sqrt()
    }

    /// Whether any dimension is changing rapidly (anomaly detection).
    pub fn has_anomalous_change(&self, threshold: f64) -> bool {
        self.velocity.iter().any(|v| v.abs() > threshold)
    }

    /// Which dimensions are changing most rapidly.
    pub fn most_volatile_dimensions(&self, top_n: usize) -> Vec<(usize, f64)> {
        let mut indexed: Vec<(usize, f64)> = self
            .velocity
            .iter()
            .enumerate()
            .map(|(i, v)| (i, v.abs()))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        indexed.truncate(top_n);
        indexed
    }

    /// Trend prediction: extrapolate K-Vector forward by given days.
    pub fn predict(&self, days_forward: f64) -> [f64; 8] {
        let current = self.current.as_array();
        let mut predicted = [0.0; 8];
        for i in 0..8 {
            predicted[i] = (current[i]
                + self.velocity[i] * days_forward
                + 0.5 * self.acceleration[i] * days_forward.powi(2))
            .clamp(0.0, 1.0);
        }
        predicted
    }
}

// =============================================================================
// Field Interference [Primitive 6]
// =============================================================================

/// Result of field interference computation between K-Vector fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInterference {
    /// Agents involved
    pub agents: Vec<String>,
    /// Per-dimension interference pattern
    pub pattern: [InterferenceDimension; 8],
    /// Overall interference type
    pub overall_type: OverallInterferenceType,
    /// Combined amplitude [0.0, 2.0] (constructive can exceed 1.0)
    pub amplitude: f64,
    pub computed_at: DateTime<Utc>,
}

/// Interference result for a single K-Vector dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceDimension {
    /// Dimension index
    pub dim: usize,
    /// Phase difference between fields [0, π]
    pub phase_difference: f64,
    /// Resulting amplitude
    pub amplitude: f64,
    /// Constructive or destructive?
    pub constructive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallInterferenceType {
    /// Mostly constructive: agents reinforce each other
    Constructive,
    /// Mostly destructive: agents cancel each other
    Destructive,
    /// Mixed: some dimensions constructive, some destructive
    Mixed,
}

impl FieldInterference {
    /// Compute interference between two K-Vector signatures.
    pub fn compute(a: &KVectorSignature, b: &KVectorSignature) -> Self {
        let va = a.as_array();
        let vb = b.as_array();

        let mut pattern = Vec::with_capacity(8);
        let mut constructive_count = 0;

        for i in 0..8 {
            // Treat values as wave amplitudes
            // Phase difference approximated by relative direction
            let phase_diff = if va[i] * vb[i] > 0.0 {
                // Same sign: in-phase (constructive)
                0.0
            } else if va[i] * vb[i] < 0.0 {
                // Opposite sign: anti-phase (destructive)
                std::f64::consts::PI
            } else {
                // One is zero: quadrature
                std::f64::consts::FRAC_PI_2
            };

            let amplitude = ((va[i] + vb[i]).abs() + (va[i] - vb[i]).abs()) / 2.0;
            let constructive = phase_diff < std::f64::consts::FRAC_PI_2;

            if constructive {
                constructive_count += 1;
            }

            pattern.push(InterferenceDimension {
                dim: i,
                phase_difference: phase_diff,
                amplitude,
                constructive,
            });
        }

        let overall_type = if constructive_count >= 6 {
            OverallInterferenceType::Constructive
        } else if constructive_count <= 2 {
            OverallInterferenceType::Destructive
        } else {
            OverallInterferenceType::Mixed
        };

        let amplitude = pattern.iter().map(|d| d.amplitude).sum::<f64>() / 8.0;

        Self {
            agents: vec![],
            pattern: pattern.try_into().unwrap(),
            overall_type,
            amplitude,
            computed_at: Utc::now(),
        }
    }
}

// =============================================================================
// Network-Level K-Vector [Primitive 8]
// =============================================================================

/// Network-level K-Vector computed during Emergent Personhood phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkKVector {
    /// Aggregate K-Vector for the entire network
    pub aggregate: KVectorSignature,
    /// Phi (integrated information) of the network
    pub phi: f64,
    /// Number of participating nodes
    pub node_count: u64,
    /// Standard deviation per dimension
    pub std_dev: [f64; 8],
    /// Spectral gap (hive coherence)
    pub spectral_k: f64,
    pub computed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_kvec(values: [f64; 8]) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now())
    }

    #[test]
    fn test_sanity_check() {
        let sane = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.8]);
        let insane = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.3]);

        assert!(sane.is_sane());
        assert!(!insane.is_sane());
    }

    #[test]
    fn test_alignment() {
        let a = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.7, 0.8]);
        let b = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.8, 0.8]);
        let c = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.2, 0.8]);

        assert!(a.is_aligned_with(&b)); // distance 0.1 < 0.3
        assert!(!a.is_aligned_with(&c)); // distance 0.5 > 0.3
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = sample_kvec([0.5, 0.6, 0.7, 0.8, 0.5, 0.6, 0.7, 0.8]);
        let sim = a.cosine_similarity(&a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_temporal_kvector_update() {
        let initial = KVectorSignature::from_array(
            [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            Utc::now() - chrono::Duration::days(1),
        );
        let mut temporal = TemporalKVector::new(initial, 100);

        let updated = KVectorSignature::from_array(
            [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            Utc::now(),
        );
        temporal.update(updated);

        // k_r changed by 0.1 in ~1 day, so velocity[0] ≈ 0.1
        assert!(temporal.velocity[0] > 0.09 && temporal.velocity[0] < 0.11);
        // Other dimensions should be ~0
        assert!(temporal.velocity[1].abs() < 0.01);
    }

    #[test]
    fn test_temporal_kvector_prediction() {
        let initial = KVectorSignature::from_array(
            [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            Utc::now() - chrono::Duration::days(1),
        );
        let mut temporal = TemporalKVector::new(initial, 100);

        let updated = KVectorSignature::from_array(
            [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            Utc::now(),
        );
        temporal.update(updated);

        let predicted = temporal.predict(1.0);
        // predicted = current + velocity * days + 0.5 * acceleration * days^2
        // = 0.6 + 0.1 * 1 + 0.5 * 0.1 * 1 = 0.75
        // (acceleration = (0.1 - 0.0) / 1.0 = 0.1 from single update)
        assert!(predicted[0] > 0.74 && predicted[0] < 0.76,
            "predicted[0] = {}", predicted[0]);
    }

    #[test]
    fn test_field_interference_constructive() {
        let a = sample_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
        let b = sample_kvec([0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]);

        let interference = FieldInterference::compute(&a, &b);
        assert_eq!(interference.overall_type, OverallInterferenceType::Constructive);
    }

    #[test]
    fn test_rate_of_change() {
        let initial = KVectorSignature::from_array(
            [0.5; 8],
            Utc::now() - chrono::Duration::days(1),
        );
        let mut temporal = TemporalKVector::new(initial, 100);
        assert_eq!(temporal.rate_of_change(), 0.0);

        let updated = KVectorSignature::from_array(
            [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            Utc::now(),
        );
        temporal.update(updated);
        assert!(temporal.rate_of_change() > 0.0);
    }
}
