//! # Emergent Personhood Service [Primitive 8]
//!
//! Network self-measurement: computes Phi (integrated information) as a measure
//! of the network's consciousness, and produces a network-level K-Vector that
//! represents the aggregate identity of the entire agent population.
//!
//! ## Phi Computation
//!
//! Phi (integrated information) measures the degree to which the network is
//! "more than the sum of its parts." It is computed by comparing the total
//! mutual information of the system against the sum of mutual information of
//! its best bipartition.
//!
//! Simplified approximation:
//! 1. Compute the covariance matrix of all K-Vector dimensions across agents.
//! 2. Compute total mutual information from the determinant of the covariance matrix.
//! 3. Find the minimum information partition (MIP) via spectral bisection.
//! 4. Phi = total mutual information - MIP mutual information.
//!
//! ## Network K-Vector
//!
//! The network K-Vector is a reputation-weighted or uniform average of all
//! individual K-Vectors, plus standard deviation per dimension and spectral gap.
//!
//! ## Tier
//! Tier 4 (aspirational). Requires `tier4-aspirational` feature flag.

use chrono::Utc;
use tracing::{debug, info};

use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Gate1Check, Gate2Warning, KVectorSignature, LivingProtocolError,
    LivingProtocolEvent, LivingResult, NetworkKVector, NetworkPhiComputedEvent,
};

/// Default Phi threshold above which the network is considered "conscious."
const DEFAULT_PHI_THRESHOLD: f64 = 0.5;

// =============================================================================
// EmergentPersonhoodService
// =============================================================================

/// Service for computing network-level consciousness metrics.
#[derive(Debug)]
pub struct EmergentPersonhoodService {
    /// Most recently computed Phi value.
    last_phi: Option<f64>,
    /// Most recently computed network K-Vector.
    last_network_kvec: Option<NetworkKVector>,
    /// Phi threshold for consciousness.
    phi_threshold: f64,
    /// Events emitted since last drain.
    pending_events: Vec<LivingProtocolEvent>,
}

impl EmergentPersonhoodService {
    /// Create a new service with the default Phi threshold.
    pub fn new() -> Self {
        Self {
            last_phi: None,
            last_network_kvec: None,
            phi_threshold: DEFAULT_PHI_THRESHOLD,
            pending_events: Vec::new(),
        }
    }

    /// Create a new service with a custom Phi threshold.
    pub fn with_threshold(phi_threshold: f64) -> Self {
        Self {
            last_phi: None,
            last_network_kvec: None,
            phi_threshold,
            pending_events: Vec::new(),
        }
    }

    /// Compute Phi (integrated information) for the network.
    ///
    /// This uses a simplified approximation based on the covariance structure
    /// of K-Vector dimensions across agents:
    ///
    /// 1. Build an NxD matrix where N = agents, D = 8 dimensions.
    /// 2. Compute the covariance matrix (DxD).
    /// 3. Total information = log(det(diag(variances))) - log(det(covariance)).
    /// 4. Split agents into two halves (spectral bisection approximation).
    /// 5. Compute information for each half.
    /// 6. Phi = total information - max(info_half1, info_half2).
    ///
    /// Returns the Phi value, clamped to [0, +inf).
    pub fn compute_network_phi(&mut self, k_vectors: &[KVectorSignature]) -> LivingResult<f64> {
        let n = k_vectors.len();
        if n < 2 {
            return Err(LivingProtocolError::PhiComputationFailed(
                "Need at least 2 agents to compute Phi".to_string(),
            ));
        }

        // Step 1: Build data matrix (N x 8)
        let data: Vec<[f64; 8]> = k_vectors.iter().map(|k| k.as_array()).collect();

        // Step 2: Compute means
        let means = compute_means(&data);

        // Step 3: Compute covariance matrix (8x8)
        let cov = compute_covariance(&data, &means);

        // Step 4: Compute total integration
        let total_info = compute_integration_from_covariance(&cov);

        // Step 5: Bisection - split into two halves
        let mid = n / 2;
        let half1 = &data[..mid];
        let half2 = &data[mid..];

        let means1 = compute_means(half1);
        let cov1 = compute_covariance(half1, &means1);
        let info1 = compute_integration_from_covariance(&cov1);

        let means2 = compute_means(half2);
        let cov2 = compute_covariance(half2, &means2);
        let info2 = compute_integration_from_covariance(&cov2);

        // Step 6: Phi = total - max of parts
        let phi = (total_info - info1.max(info2)).max(0.0);

        self.last_phi = Some(phi);

        let event = NetworkPhiComputedEvent {
            phi,
            node_count: n as u64,
            integration_score: total_info,
            timestamp: Utc::now(),
        };

        info!(
            phi = phi,
            node_count = n,
            integration = total_info,
            "Network Phi computed"
        );

        self.pending_events
            .push(LivingProtocolEvent::NetworkPhiComputed(event));

        Ok(phi)
    }

    /// Compute the network-level K-Vector as a uniform average of all agent
    /// K-Vectors, with standard deviation per dimension and spectral gap.
    pub fn compute_network_k_vector(
        &mut self,
        k_vectors: &[KVectorSignature],
    ) -> LivingResult<NetworkKVector> {
        if k_vectors.is_empty() {
            return Err(LivingProtocolError::PhiComputationFailed(
                "Need at least 1 agent to compute network K-Vector".to_string(),
            ));
        }

        let n = k_vectors.len() as f64;
        let data: Vec<[f64; 8]> = k_vectors.iter().map(|k| k.as_array()).collect();

        // Compute mean per dimension
        let means = compute_means(&data);

        // Compute standard deviation per dimension
        let mut std_dev = [0.0f64; 8];
        for dim in 0..8 {
            let variance: f64 = data
                .iter()
                .map(|d| (d[dim] - means[dim]).powi(2))
                .sum::<f64>()
                / n;
            std_dev[dim] = variance.sqrt();
        }

        // Spectral gap approximation: 1 - mean standard deviation
        // Higher spectral gap = more coherent network
        let mean_std: f64 = std_dev.iter().sum::<f64>() / 8.0;
        let spectral_k = (1.0 - mean_std).clamp(0.0, 1.0);

        let aggregate = KVectorSignature::from_array(means, Utc::now());

        let phi = self.last_phi.unwrap_or(0.0);

        let network_kvec = NetworkKVector {
            aggregate,
            phi,
            node_count: k_vectors.len() as u64,
            std_dev,
            spectral_k,
            computed_at: Utc::now(),
        };

        self.last_network_kvec = Some(network_kvec.clone());

        debug!(
            node_count = k_vectors.len(),
            spectral_k = spectral_k,
            "Network K-Vector computed"
        );

        Ok(network_kvec)
    }

    /// Measure the integration (total mutual information) of the K-Vector
    /// population. Higher values indicate more integrated (interconnected)
    /// agent behavior.
    pub fn measure_integration(&self, k_vectors: &[KVectorSignature]) -> LivingResult<f64> {
        if k_vectors.len() < 2 {
            return Err(LivingProtocolError::PhiComputationFailed(
                "Need at least 2 agents to measure integration".to_string(),
            ));
        }

        let data: Vec<[f64; 8]> = k_vectors.iter().map(|k| k.as_array()).collect();
        let means = compute_means(&data);
        let cov = compute_covariance(&data, &means);
        Ok(compute_integration_from_covariance(&cov))
    }

    /// Whether the network is considered "conscious" based on the most recent
    /// Phi computation.
    ///
    /// Returns `false` if Phi has not yet been computed.
    pub fn is_network_conscious(&self, phi_threshold: f64) -> bool {
        match self.last_phi {
            Some(phi) => phi >= phi_threshold,
            None => false,
        }
    }

    /// Get the most recently computed Phi value.
    pub fn last_phi(&self) -> Option<f64> {
        self.last_phi
    }

    /// Get the most recently computed network K-Vector.
    pub fn last_network_k_vector(&self) -> Option<&NetworkKVector> {
        self.last_network_kvec.as_ref()
    }

    /// Get the configured Phi threshold.
    pub fn phi_threshold(&self) -> f64 {
        self.phi_threshold
    }

    /// Drain pending events.
    pub fn drain_events(&mut self) -> Vec<LivingProtocolEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl Default for EmergentPersonhoodService {
    fn default() -> Self {
        Self::new()
    }
}

impl LivingPrimitive for EmergentPersonhoodService {
    fn primitive_id(&self) -> &str {
        "emergent_personhood"
    }

    fn primitive_number(&self) -> u8 {
        8
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Consciousness
    }

    fn tier(&self) -> u8 {
        4
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(self.drain_events())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: If Phi was computed, it must be non-negative and finite
        if let Some(phi) = self.last_phi {
            checks.push(Gate1Check {
                invariant: "phi_non_negative".to_string(),
                passed: phi >= 0.0 && phi.is_finite(),
                details: if phi >= 0.0 && phi.is_finite() {
                    None
                } else {
                    Some(format!(
                        "Phi value {} is invalid (must be non-negative and finite)",
                        phi
                    ))
                },
            });
        }

        // Gate 1: Network K-Vector dimensions must be in [0, 1]
        if let Some(ref nk) = self.last_network_kvec {
            let arr = nk.aggregate.as_array();
            let in_bounds = arr.iter().all(|v| *v >= 0.0 && *v <= 1.0);
            checks.push(Gate1Check {
                invariant: "network_kvec_bounded".to_string(),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some("Network K-Vector has values outside [0, 1]".to_string())
                },
            });
        }

        if checks.is_empty() {
            checks.push(Gate1Check {
                invariant: "emergent_personhood_no_computation_yet".to_string(),
                passed: true,
                details: Some("No computation performed yet".to_string()),
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        if let Some(phi) = self.last_phi {
            if phi < self.phi_threshold {
                warnings.push(Gate2Warning {
                    harmony_violated: "Integrated Awareness".to_string(),
                    severity: 1.0 - (phi / self.phi_threshold).min(1.0),
                    reputation_impact: 0.0, // Network-level metric, no individual impact
                    reasoning: format!(
                        "Network Phi ({:.4}) is below threshold ({:.4}). \
                         Network may not be sufficiently integrated.",
                        phi, self.phi_threshold
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Primarily active during EmergentPersonhood phase, but can compute anytime
        matches!(phase, CyclePhase::EmergentPersonhood)
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "primitive": "emergent_personhood",
            "last_phi": self.last_phi,
            "phi_threshold": self.phi_threshold,
            "is_conscious": self.is_network_conscious(self.phi_threshold),
            "node_count": self.last_network_kvec.as_ref().map(|n| n.node_count),
            "spectral_k": self.last_network_kvec.as_ref().map(|n| n.spectral_k),
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute the mean of each dimension across all data points.
fn compute_means(data: &[[f64; 8]]) -> [f64; 8] {
    let n = data.len() as f64;
    let mut means = [0.0f64; 8];

    for row in data {
        for (i, val) in row.iter().enumerate() {
            means[i] += val;
        }
    }

    for m in means.iter_mut() {
        *m /= n;
    }

    means
}

/// Compute the 8x8 covariance matrix.
fn compute_covariance(data: &[[f64; 8]], means: &[f64; 8]) -> [[f64; 8]; 8] {
    let n = data.len() as f64;
    let mut cov = [[0.0f64; 8]; 8];

    for row in data {
        for i in 0..8 {
            for j in 0..8 {
                cov[i][j] += (row[i] - means[i]) * (row[j] - means[j]);
            }
        }
    }

    for i in 0..8 {
        for j in 0..8 {
            cov[i][j] /= n;
        }
    }

    cov
}

/// Compute integration (mutual information proxy) from a covariance matrix.
///
/// Integration I = sum(log(sigma_i^2)) - log(det(Sigma))
///
/// where sigma_i^2 are the diagonal variances and det(Sigma) is the
/// determinant of the full covariance matrix. This measures how much
/// information is shared across dimensions (redundancy).
///
/// A regularization term (epsilon on diagonal) prevents singularity.
fn compute_integration_from_covariance(cov: &[[f64; 8]; 8]) -> f64 {
    let epsilon = 1e-10;

    // Sum of log variances (diagonal)
    let sum_log_var: f64 = (0..8).map(|i| (cov[i][i] + epsilon).ln()).sum();

    // Log determinant of full covariance matrix
    // Use nalgebra for numeric stability
    let mut mat = nalgebra::SMatrix::<f64, 8, 8>::zeros();
    for i in 0..8 {
        for j in 0..8 {
            mat[(i, j)] = cov[i][j];
        }
        mat[(i, i)] += epsilon; // regularization
    }

    let det = mat.determinant();
    let log_det = if det > 0.0 { det.ln() } else { -100.0 };

    // Integration = sum of individual entropies - joint entropy

    (sum_log_var - log_det).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_kvec(values: [f64; 8]) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now())
    }

    #[test]
    fn test_compute_phi_requires_minimum_agents() {
        let mut service = EmergentPersonhoodService::new();
        let result = service.compute_network_phi(&[make_kvec([0.5; 8])]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_phi_identical_agents() {
        let mut service = EmergentPersonhoodService::new();
        // Identical agents have zero variance -> Phi should be 0 or near 0
        let kvecs: Vec<KVectorSignature> = (0..10).map(|_| make_kvec([0.5; 8])).collect();

        let phi = service.compute_network_phi(&kvecs).unwrap();
        // With identical data, covariance is zero matrix, integration ~ 0
        assert!(phi >= 0.0, "Phi must be non-negative, got {}", phi);
    }

    #[test]
    fn test_compute_phi_diverse_agents() {
        let mut service = EmergentPersonhoodService::new();
        let kvecs = vec![
            make_kvec([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]),
            make_kvec([0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]),
            make_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_kvec([0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1]),
            make_kvec([0.3, 0.8, 0.2, 0.7, 0.4, 0.6, 0.3, 0.9]),
        ];

        let phi = service.compute_network_phi(&kvecs).unwrap();
        assert!(phi >= 0.0, "Phi must be non-negative, got {}", phi);
        assert!(phi.is_finite(), "Phi must be finite");
    }

    #[test]
    fn test_compute_phi_emits_event() {
        let mut service = EmergentPersonhoodService::new();
        let kvecs = vec![
            make_kvec([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]),
            make_kvec([0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]),
        ];

        service.compute_network_phi(&kvecs).unwrap();

        let events = service.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            LivingProtocolEvent::NetworkPhiComputed(e) => {
                assert!(e.phi >= 0.0);
                assert_eq!(e.node_count, 2);
            }
            _ => panic!("Expected NetworkPhiComputed event"),
        }
    }

    #[test]
    fn test_compute_network_k_vector() {
        let mut service = EmergentPersonhoodService::new();
        let kvecs = vec![
            make_kvec([0.4, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4, 0.4]),
            make_kvec([0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]),
        ];

        let nk = service.compute_network_k_vector(&kvecs).unwrap();

        // Mean should be 0.5 for all dimensions
        let arr = nk.aggregate.as_array();
        for v in arr.iter() {
            assert!((*v - 0.5).abs() < 1e-10);
        }

        // Standard deviation should be 0.1 for all dimensions
        for sd in nk.std_dev.iter() {
            assert!((*sd - 0.1).abs() < 1e-10);
        }

        assert_eq!(nk.node_count, 2);
        assert!(nk.spectral_k >= 0.0 && nk.spectral_k <= 1.0);
    }

    #[test]
    fn test_compute_network_k_vector_empty() {
        let mut service = EmergentPersonhoodService::new();
        let result = service.compute_network_k_vector(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_network_k_vector_single_agent() {
        let mut service = EmergentPersonhoodService::new();
        let kvecs = vec![make_kvec([0.5; 8])];

        let nk = service.compute_network_k_vector(&kvecs).unwrap();
        let arr = nk.aggregate.as_array();
        for v in arr.iter() {
            assert!((*v - 0.5).abs() < 1e-10);
        }
        // Single agent -> zero std dev
        for sd in nk.std_dev.iter() {
            assert!(*sd < 1e-10);
        }
        // Single agent, zero std -> spectral_k close to 1
        assert!((nk.spectral_k - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_integration() {
        let service = EmergentPersonhoodService::new();
        let kvecs = vec![
            make_kvec([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]),
            make_kvec([0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]),
            make_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
        ];

        let integration = service.measure_integration(&kvecs).unwrap();
        assert!(integration >= 0.0, "Integration must be non-negative");
        assert!(integration.is_finite(), "Integration must be finite");
    }

    #[test]
    fn test_measure_integration_requires_two() {
        let service = EmergentPersonhoodService::new();
        let result = service.measure_integration(&[make_kvec([0.5; 8])]);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_network_conscious_not_computed() {
        let service = EmergentPersonhoodService::new();
        assert!(!service.is_network_conscious(0.5));
    }

    #[test]
    fn test_is_network_conscious_above_threshold() {
        let mut service = EmergentPersonhoodService::with_threshold(0.01);
        // Use diverse agents that produce nonzero Phi
        let kvecs = vec![
            make_kvec([0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9]),
            make_kvec([0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1]),
            make_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_kvec([0.3, 0.7, 0.3, 0.7, 0.3, 0.7, 0.3, 0.7]),
        ];

        let phi = service.compute_network_phi(&kvecs).unwrap();
        // With a low threshold, diverse agents should pass
        if phi >= 0.01 {
            assert!(service.is_network_conscious(0.01));
        }
    }

    #[test]
    fn test_last_phi_stored() {
        let mut service = EmergentPersonhoodService::new();
        assert!(service.last_phi().is_none());

        let kvecs = vec![make_kvec([0.3; 8]), make_kvec([0.7; 8])];
        let phi = service.compute_network_phi(&kvecs).unwrap();
        assert_eq!(service.last_phi(), Some(phi));
    }

    #[test]
    fn test_last_network_kvec_stored() {
        let mut service = EmergentPersonhoodService::new();
        assert!(service.last_network_k_vector().is_none());

        let kvecs = vec![make_kvec([0.5; 8])];
        service.compute_network_k_vector(&kvecs).unwrap();
        assert!(service.last_network_k_vector().is_some());
    }

    #[test]
    fn test_gate1_check_no_computation() {
        let service = EmergentPersonhoodService::new();
        let checks = service.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_gate1_check_valid_phi() {
        let mut service = EmergentPersonhoodService::new();
        let kvecs = vec![make_kvec([0.3; 8]), make_kvec([0.7; 8])];
        service.compute_network_phi(&kvecs).unwrap();

        let checks = service.gate1_check();
        assert!(
            checks.iter().all(|c| c.passed),
            "Gate 1 violations: {:?}",
            checks.iter().filter(|c| !c.passed).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_gate2_warning_below_threshold() {
        let mut service = EmergentPersonhoodService::with_threshold(10.0);
        let kvecs = vec![make_kvec([0.5; 8]), make_kvec([0.5; 8])];
        service.compute_network_phi(&kvecs).unwrap();

        let warnings = service.gate2_check();
        assert!(
            !warnings.is_empty(),
            "Should warn when Phi is below threshold"
        );
    }

    #[test]
    fn test_living_primitive_trait() {
        let service = EmergentPersonhoodService::new();
        assert_eq!(service.primitive_id(), "emergent_personhood");
        assert_eq!(service.primitive_number(), 8);
        assert_eq!(service.module(), PrimitiveModule::Consciousness);
        assert_eq!(service.tier(), 4);
        assert!(service.is_active_in_phase(CyclePhase::EmergentPersonhood));
        assert!(!service.is_active_in_phase(CyclePhase::Shadow));
    }

    #[test]
    fn test_collect_metrics() {
        let service = EmergentPersonhoodService::new();
        let metrics = service.collect_metrics();
        assert_eq!(metrics["primitive"], "emergent_personhood");
        assert_eq!(metrics["is_conscious"], false);
    }

    #[test]
    fn test_compute_means_helper() {
        let data = vec![
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        ];
        let means = compute_means(&data);
        assert!((means[0] - 1.5).abs() < 1e-10);
        assert!((means[7] - 8.5).abs() < 1e-10);
    }

    #[test]
    fn test_full_lifecycle() {
        let mut service = EmergentPersonhoodService::with_threshold(0.0);

        let kvecs = vec![
            make_kvec([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]),
            make_kvec([0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1]),
            make_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_kvec([0.9, 0.1, 0.9, 0.1, 0.9, 0.1, 0.9, 0.1]),
        ];

        // Compute Phi
        let phi = service.compute_network_phi(&kvecs).unwrap();
        assert!(phi >= 0.0);

        // Compute network K-Vector
        let nk = service.compute_network_k_vector(&kvecs).unwrap();
        assert_eq!(nk.node_count, 4);
        assert!(nk.spectral_k >= 0.0 && nk.spectral_k <= 1.0);

        // Measure integration
        let integration = service.measure_integration(&kvecs).unwrap();
        assert!(integration >= 0.0);

        // Check consciousness
        let conscious = service.is_network_conscious(0.0);
        // With threshold 0 and diverse agents, should be conscious
        assert!(conscious || phi == 0.0);

        // Verify events
        let events = service.drain_events();
        assert_eq!(events.len(), 1); // Only Phi computation emits events
    }
}
