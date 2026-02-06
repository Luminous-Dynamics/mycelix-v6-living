//! # Resonance Addressing Engine -- Primitive [17]
//!
//! Pattern-based routing replacing hash addressing.
//!
//! Instead of purely hash-based DHT lookup, resonance addressing routes by
//! semantic pattern similarity.  Each address carries a semantic embedding and
//! harmonic signature computed from its content.  Resolution can proceed either
//! by exact hash match or by finding addresses whose semantic embeddings exceed
//! a cosine similarity threshold.
//!
//! ## Constitutional Alignment
//!
//! **Evolutionary Progression (Harmony 7)**: Content discovery should not be
//! limited to those who already know the exact hash.  Pattern-based resolution
//! enables serendipitous discovery, strengthening the network's capacity for
//! creative recombination.
//!
//! ## Three Gates
//!
//! - **Gate 1**: `semantic_embedding` and `harmonic_signature` must be non-empty.
//! - **Gate 2**: Warns if an address has zero semantic similarity to all existing
//!   addresses (isolated address).
//!
//! ## Dependency
//!
//! Depends on [5] Temporal K-Vector for pattern correlation across time.
//!
//! ## Classification
//!
//! E3/N0/M0 -- Cryptographically proven / Personal / Ephemeral.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use living_core::error::LivingResult;
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Did, EventBus, Gate1Check, Gate2Warning, HashDigest, LivingProtocolEvent,
    ResonanceAddress, ResonanceAddressCreatedEvent,
};
use sha2::{Digest, Sha256};

// =============================================================================
// Resonance Address Entry
// =============================================================================

/// A registered resonance address with its owner and content identifier.
#[derive(Debug, Clone)]
pub struct ResonanceAddressEntry {
    /// The resonance address itself.
    pub address: ResonanceAddress,
    /// DID of the address owner.
    pub owner_did: Did,
    /// Content identifier string (human-readable label or original content).
    pub content_id: String,
}

// =============================================================================
// Resonance Addressing Engine
// =============================================================================

/// Engine for creating, resolving, and managing resonance addresses.
///
/// Pattern-based routing replaces pure hash-based DHT lookup.  Addresses are
/// resolved either by exact hash or by semantic similarity.
pub struct ResonanceAddressingEngine {
    /// Registered addresses indexed by their pattern hash.
    addresses: HashMap<HashDigest, ResonanceAddressEntry>,
    /// Event bus for emitting resonance addressing events.
    event_bus: Arc<dyn EventBus>,
    /// Whether the engine is active in the current cycle phase.
    active: bool,
}

impl ResonanceAddressingEngine {
    /// Create a new resonance addressing engine.
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            addresses: HashMap::new(),
            event_bus,
            active: false,
        }
    }

    /// Create a resonance address for the given content and owner.
    ///
    /// Computes the semantic embedding and harmonic signature from the content,
    /// then derives the pattern hash from both vectors.  Emits a
    /// `ResonanceAddressCreated` event and returns it.
    pub fn create_address(
        &mut self,
        content: &str,
        owner_did: Did,
    ) -> ResonanceAddressCreatedEvent {
        let semantic_embedding = Self::compute_semantic_embedding(content);
        let harmonic_signature = Self::compute_harmonic_signature(content);
        let pattern_hash = Self::compute_pattern_hash(&semantic_embedding, &harmonic_signature);
        let now = Utc::now();

        let address = ResonanceAddress {
            pattern_hash,
            semantic_embedding,
            harmonic_signature,
            created: now,
        };

        let entry = ResonanceAddressEntry {
            address: address.clone(),
            owner_did: owner_did.clone(),
            content_id: content.to_string(),
        };

        self.addresses.insert(pattern_hash, entry);

        let event = ResonanceAddressCreatedEvent {
            address,
            owner_did,
            timestamp: now,
        };

        self.event_bus
            .publish(LivingProtocolEvent::ResonanceAddressCreated(event.clone()));

        tracing::info!(
            pattern_hash = ?pattern_hash,
            "Resonance address created. Pattern-based routing: content discoverable by semantic similarity."
        );

        event
    }

    /// Resolve addresses by semantic pattern similarity.
    ///
    /// Returns all entries whose semantic embedding has a cosine similarity to
    /// `pattern` that meets or exceeds `threshold`.  This is the core of
    /// pattern-based routing: callers do not need the exact hash, only a
    /// semantically similar query pattern.
    pub fn resolve_by_pattern(
        &self,
        pattern: &[f64],
        threshold: f64,
    ) -> Vec<ResonanceAddressEntry> {
        self.addresses
            .values()
            .filter(|entry| {
                let similarity = cosine_similarity(pattern, &entry.address.semantic_embedding);
                similarity >= threshold
            })
            .cloned()
            .collect()
    }

    /// Resolve an address by its exact pattern hash.
    pub fn resolve_by_hash(&self, hash: HashDigest) -> Option<&ResonanceAddressEntry> {
        self.addresses.get(&hash)
    }

    /// Compute a simple bag-of-words semantic embedding from content.
    ///
    /// This is a placeholder implementation.  In production this would use a
    /// proper embedding model.  The current approach:
    /// 1. Lowercase and split into words.
    /// 2. Hash each word to a dimension index in a fixed-size vector.
    /// 3. Accumulate counts, then L2-normalize.
    pub fn compute_semantic_embedding(content: &str) -> Vec<f64> {
        const DIMS: usize = 64;
        let mut embedding = vec![0.0_f64; DIMS];

        for word in content.to_lowercase().split_whitespace() {
            let mut hasher = Sha256::new();
            hasher.update(word.as_bytes());
            let hash = hasher.finalize();
            let idx = (hash[0] as usize) % DIMS;
            embedding[idx] += 1.0;
        }

        // L2-normalize
        let magnitude: f64 = embedding.iter().map(|v| v * v).sum::<f64>().sqrt();
        if magnitude > 0.0 {
            for v in &mut embedding {
                *v /= magnitude;
            }
        }

        embedding
    }

    /// Compute a harmonic frequency signature from content.
    ///
    /// Approximates harmonic analysis by treating byte values as samples and
    /// computing a simple DFT over a small number of frequency bins.
    pub fn compute_harmonic_signature(content: &str) -> Vec<f64> {
        const BINS: usize = 16;
        let bytes = content.as_bytes();
        let n = bytes.len() as f64;

        if bytes.is_empty() {
            return vec![0.0; BINS];
        }

        let mut signature = Vec::with_capacity(BINS);
        for k in 0..BINS {
            let mut real = 0.0_f64;
            let mut imag = 0.0_f64;
            for (i, &b) in bytes.iter().enumerate() {
                let angle = 2.0 * std::f64::consts::PI * (k as f64) * (i as f64) / n;
                real += (b as f64) * angle.cos();
                imag += (b as f64) * angle.sin();
            }
            let magnitude = (real * real + imag * imag).sqrt() / n;
            signature.push(magnitude);
        }

        // L2-normalize
        let mag: f64 = signature.iter().map(|v| v * v).sum::<f64>().sqrt();
        if mag > 0.0 {
            for v in &mut signature {
                *v /= mag;
            }
        }

        signature
    }

    /// Remove an address by its pattern hash.  Returns true if the address
    /// existed and was removed.
    pub fn remove_address(&mut self, hash: HashDigest) -> bool {
        self.addresses.remove(&hash).is_some()
    }

    /// Get the total number of registered addresses.
    pub fn address_count(&self) -> usize {
        self.addresses.len()
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Compute the pattern hash from semantic embedding and harmonic signature.
    fn compute_pattern_hash(embedding: &[f64], harmonic: &[f64]) -> HashDigest {
        let mut hasher = Sha256::new();

        // Hash embedding bytes
        for v in embedding {
            hasher.update(v.to_le_bytes());
        }
        // Hash harmonic bytes
        for v in harmonic {
            hasher.update(v.to_le_bytes());
        }

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for ResonanceAddressingEngine {
    fn primitive_id(&self) -> &str {
        "resonance_addressing"
    }

    fn primitive_number(&self) -> u8 {
        17
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Structural
    }

    fn tier(&self) -> u8 {
        2
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Resonance addressing is available in all phases but primarily used
        // during Co-Creation when new content is published.
        self.active = new_phase == CyclePhase::CoCreation;
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        for (hash, entry) in &self.addresses {
            // Gate 1: semantic embedding must be non-empty
            let embedding_ok = !entry.address.semantic_embedding.is_empty();
            checks.push(Gate1Check {
                invariant: format!("semantic_embedding non-empty for address {:?}", &hash[..4]),
                passed: embedding_ok,
                details: if embedding_ok {
                    None
                } else {
                    Some("semantic_embedding is empty".to_string())
                },
            });

            // Gate 1: harmonic signature must be non-empty
            let harmonic_ok = !entry.address.harmonic_signature.is_empty();
            checks.push(Gate1Check {
                invariant: format!("harmonic_signature non-empty for address {:?}", &hash[..4]),
                passed: harmonic_ok,
                details: if harmonic_ok {
                    None
                } else {
                    Some("harmonic_signature is empty".to_string())
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Gate 2: warn if an address is semantically isolated (no similarity
        // above 0.1 with any other address)
        for (hash, entry) in &self.addresses {
            let has_neighbor = self.addresses.values().any(|other| {
                if other.address.pattern_hash == *hash {
                    return false;
                }
                cosine_similarity(
                    &entry.address.semantic_embedding,
                    &other.address.semantic_embedding,
                ) > 0.1
            });

            if !has_neighbor && self.addresses.len() > 1 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Evolutionary Progression (Harmony 7)".to_string(),
                    severity: 0.2,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Address {:?} is semantically isolated from all other addresses. \
                         Pattern-based routing cannot discover it through similarity.",
                        &hash[..4]
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Resonance addressing is always available but prioritized during CoCreation
        phase == CyclePhase::CoCreation
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "primitive": "resonance_addressing",
            "primitive_number": 17,
            "total_addresses": self.addresses.len(),
        })
    }
}

// =============================================================================
// Utility functions
// =============================================================================

/// Cosine similarity between two vectors.  Returns 0.0 for zero-magnitude
/// vectors or vectors of different lengths.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::InMemoryEventBus;

    fn make_engine() -> ResonanceAddressingEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        ResonanceAddressingEngine::new(bus)
    }

    fn make_engine_with_bus() -> (ResonanceAddressingEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = ResonanceAddressingEngine::new(bus.clone());
        (engine, bus)
    }

    #[test]
    fn test_create_address_registers_entry() {
        let mut engine = make_engine();
        let event = engine.create_address(
            "distributed governance protocol",
            "did:mycelix:alice".to_string(),
        );

        assert_eq!(engine.address_count(), 1);
        assert!(!event.address.semantic_embedding.is_empty());
        assert!(!event.address.harmonic_signature.is_empty());
        assert_eq!(event.owner_did, "did:mycelix:alice");
    }

    #[test]
    fn test_resolve_by_hash() {
        let mut engine = make_engine();
        let event =
            engine.create_address("mycelial network topology", "did:mycelix:bob".to_string());

        let resolved = engine.resolve_by_hash(event.address.pattern_hash);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().owner_did, "did:mycelix:bob");
    }

    #[test]
    fn test_resolve_by_pattern_similar_content() {
        let mut engine = make_engine();

        engine.create_address(
            "distributed governance protocol for networks",
            "did:mycelix:alice".to_string(),
        );
        engine.create_address(
            "distributed governance mechanism for networks",
            "did:mycelix:bob".to_string(),
        );
        engine.create_address(
            "unrelated quantum physics experiment results",
            "did:mycelix:carol".to_string(),
        );

        // Query with a pattern similar to the governance content
        let query = ResonanceAddressingEngine::compute_semantic_embedding(
            "distributed governance protocol",
        );

        // Use a moderate threshold to find semantically similar addresses
        let results = engine.resolve_by_pattern(&query, 0.5);
        // The two governance-related entries should match; the physics one may or may not
        assert!(
            results.len() >= 1,
            "Expected at least 1 match, got {}",
            results.len()
        );
    }

    #[test]
    fn test_resolve_by_pattern_with_high_threshold() {
        let mut engine = make_engine();

        engine.create_address("alpha beta gamma", "did:mycelix:alice".to_string());
        engine.create_address(
            "completely different words here",
            "did:mycelix:bob".to_string(),
        );

        let query = ResonanceAddressingEngine::compute_semantic_embedding("alpha beta gamma");
        // Exact self-similarity should yield 1.0, so threshold just below works
        let results = engine.resolve_by_pattern(&query, 0.99);
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_resolve_by_pattern_no_match() {
        let mut engine = make_engine();
        engine.create_address("apples oranges bananas", "did:mycelix:alice".to_string());

        // Extremely different pattern
        let query = vec![0.0; 64];
        let results = engine.resolve_by_pattern(&query, 0.9);
        assert!(results.is_empty());
    }

    #[test]
    fn test_remove_address() {
        let mut engine = make_engine();
        let event = engine.create_address("test content", "did:mycelix:alice".to_string());
        assert_eq!(engine.address_count(), 1);

        let removed = engine.remove_address(event.address.pattern_hash);
        assert!(removed);
        assert_eq!(engine.address_count(), 0);

        // Double remove returns false
        let removed_again = engine.remove_address(event.address.pattern_hash);
        assert!(!removed_again);
    }

    #[test]
    fn test_semantic_embedding_deterministic() {
        let e1 = ResonanceAddressingEngine::compute_semantic_embedding("hello world");
        let e2 = ResonanceAddressingEngine::compute_semantic_embedding("hello world");
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_semantic_embedding_normalized() {
        let embedding =
            ResonanceAddressingEngine::compute_semantic_embedding("the quick brown fox");
        let magnitude: f64 = embedding.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-10,
            "Embedding should be L2-normalized, got magnitude {}",
            magnitude
        );
    }

    #[test]
    fn test_harmonic_signature_deterministic() {
        let s1 = ResonanceAddressingEngine::compute_harmonic_signature("test");
        let s2 = ResonanceAddressingEngine::compute_harmonic_signature("test");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_harmonic_signature_normalized() {
        let sig = ResonanceAddressingEngine::compute_harmonic_signature("some content here");
        let magnitude: f64 = sig.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-10,
            "Harmonic signature should be L2-normalized, got magnitude {}",
            magnitude
        );
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_events_emitted_on_create() {
        let (mut engine, bus) = make_engine_with_bus();
        engine.create_address("test content", "did:mycelix:alice".to_string());
        assert_eq!(bus.event_count(), 1);
    }

    #[test]
    fn test_gate1_checks_pass_normal() {
        let mut engine = make_engine();
        engine.create_address("normal content", "did:mycelix:alice".to_string());

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "resonance_addressing");
        assert_eq!(engine.primitive_number(), 17);
        assert_eq!(engine.module(), PrimitiveModule::Structural);
        assert_eq!(engine.tier(), 2);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        engine.create_address("content", "did:mycelix:alice".to_string());
        let metrics = engine.collect_metrics();
        assert_eq!(metrics["total_addresses"], 1);
        assert_eq!(metrics["primitive_number"], 17);
    }
}
