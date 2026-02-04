# Mycelix v6.0 / v5.2 Integration Architecture

This document describes how the v6.0 Living Protocol Layer integrates with
the existing v5.2 mycelix-property infrastructure.

## DNA Architecture

Two Holochain DNAs run side-by-side in a single hApp:

| DNA | Source | Purpose |
|-----|--------|---------|
| `mycelix-property` | v5.2 repo | Core DKG, governance, MATL, slashing |
| `mycelix-living-protocol` | this repo | 21 living primitives, cycle engine |

Cross-DNA calls use `call_remote` for coordinator-to-coordinator communication
and shared types from `mycelix_shared`.

## K-Vector Extension

The v6.0 `TemporalKVector` wraps the existing v5.2 `KVectorSignature`:

```
v5.2 KVectorSignature (5 dimensions)
  └─ v6.0 TemporalKVector (derivatives, rate-of-change, anomaly detection)
```

- v5.2 continues to produce base K-Vector snapshots.
- v6.0 `TemporalKVectorService` consumes snapshots and computes derivatives,
  velocity, acceleration, and predictions.
- No changes to v5.2 K-Vector computation are required.

## MATL -> MetabolicTrust Data Flow

```
v5.2 MATL Score
  ├─ Throughput component ──> v6.0 MetabolicTrustEngine.update_throughput()
  ├─ Resilience component ─> v6.0 MetabolicTrustEngine.update_resilience()
  └─ Composting component ─> v6.0 MetabolicTrustEngine.update_composting_contribution()

v6.0 MetabolicTrust = weighted(MATL, throughput, resilience, composting)
```

The v5.2 MATL score is one input to the v6.0 metabolic trust computation.
A cross-DNA zome call from `living_metabolism` coordinator retrieves the
MATL score from the `mycelix-property` DNA.

## Slashing -> Wound Healing Migration

v5.2 uses punitive slashing. v6.0 replaces this with the 4-phase wound
healing model:

| v5.2 Slash % | v6.0 Severity | Healing Arc |
|-------------|--------------|-------------|
| 1-5% | Minor | Short hemostasis, minimal restitution |
| 5-15% | Moderate | Standard 4-phase healing |
| 15-30% | Severe | Extended hemostasis, scar tissue |
| 30%+ | Critical | Maximum healing cycle, mandatory scar |

**Migration path:**
1. v5.2 slash events are intercepted by a bridge zome.
2. Bridge creates a `WoundRecord` in the living-protocol DNA instead of
   executing the slash.
3. Escrowed funds are held on-chain via `WoundEscrow.sol`.
4. The agent progresses through the healing arc (Hemostasis -> Inflammation
   -> Proliferation -> Remodeling -> Healed).
5. Scar tissue formation strengthens the healed area.

During the migration period, both systems can run in parallel with a
feature flag controlling which path is active.

## Governance -> BeautyScoring Integration

v5.2 governance proposals gain an additional scoring dimension in v6.0:

1. Proposal submitted through v5.2 governance.
2. During the Beauty phase (2 days of the 28-day cycle),
   `BeautyValidityEngine` scores proposals on 5 aesthetic dimensions.
3. Beauty score is attached as metadata to the v5.2 governance record.
4. Proposals below the `minimum_beauty_threshold` emit a Gate 2 warning
   (advisory, not blocking).

## Cross-DNA Zome Calls

| Caller | Callee | Purpose |
|--------|--------|---------|
| `living_metabolism` | `mycelix-property::governance` | Read MATL score |
| `living_metabolism` | `mycelix-property::slashing` | Intercept slash -> wound |
| `living_epistemics` | `mycelix-property::governance` | Attach beauty scores |
| `living_consciousness` | `mycelix-property::k_vector` | Read K-Vector snapshots |
| `living_relational` | `mycelix-property::agent_registry` | Resolve DIDs |
| `living_structural` | `mycelix-property::dht` | Resonance address resolution |

## Shared Dependency Management

Both repositories share workspace dependencies through compatible version
pins. Key shared crates:

- `serde` / `serde_json` — serialization
- `hdk` / `hdi` — Holochain SDK
- `holo_hash` — agent and entry addressing

The `mycelix_shared` zome crate contains types used by both DNAs:
`Did`, `EpistemicClassification`, `CyclePhase`, `PresenceProof`.

## Event Flow

```
v6.0 Cycle Engine
  │
  ├─ on_enter(phase) ──> Phase handler ──> Primitive engine
  ├─ on_tick(phase)  ──> Phase handler ──> Primitive engine
  └─ on_exit(phase)  ──> Phase handler ──> Primitive engine
                              │
                              ├─ LivingProtocolEvent ──> InMemoryEventBus
                              │                              │
                              │                              └─> Cross-DNA bridge
                              │                                   └─> v5.2 event handlers
                              └─ Metrics ──> collect_metrics() ──> Dashboard
```

## Deployment Order

1. Deploy `mycelix-property` DNA (v5.2, unchanged).
2. Deploy `mycelix-living-protocol` DNA alongside it.
3. Install bridge zomes for cross-DNA calls.
4. Enable living protocol features incrementally via `FeatureFlags`.
5. Run both slashing and wound healing in parallel during migration.
6. Disable v5.2 slashing once wound healing is validated.
