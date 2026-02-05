# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Property-based fuzzing tests for invariants
- Criterion benchmarks for performance testing
- Grafana dashboard templates
- Prometheus alerting rules
- Example dApp integration
- CLI tool for local development
- v5.2 migration scripts
- Comprehensive README and documentation

### Changed
- Improved TypeScript SDK test coverage
- Enhanced CI/CD pipeline with release automation

### Fixed
- Clippy warnings across all crates

## [0.6.0] - 2026-02-05

### Added

#### Core Infrastructure
- `living-core` crate with shared types, events, and error handling
- Event bus pattern with `InMemoryEventBus`
- Configuration system with `LivingProtocolConfig`
- Feature flags for experimental/aspirational primitives

#### Metabolism Module (Primitives 1-4)
- `CompostingEngine` - Decompose outdated patterns for renewal
- `WoundHealingEngine` - 4-phase biological healing model
- `MetabolicTrustEngine` - Living, earned trust between agents
- `KenosisEngine` - Irrevocable self-emptying (max 20%/cycle)

#### Consciousness Module (Primitives 5-8)
- `TemporalKVectorService` - 8D consciousness state with derivatives
- `FieldInterferenceCalculator` - Constructive/destructive overlap
- `CollectiveDreamingService` - Liminal-state proposals
- `EmergentPersonhoodService` - Network-level Φ measurement

#### Epistemics Module (Primitives 9-12)
- `ShadowIntegrationEngine` - Surface hidden/repressed patterns
- `NegativeCapabilityEngine` - Hold in uncertainty
- `SilenceSignalService` - Collective silence as epistemic act
- `BeautyValidityEngine` - 5D aesthetic evaluation

#### Relational Module (Primitives 13-16)
- `EntanglementEngine` - Quantum-inspired relational bonding
- `ErosAttractorEngine` - Co-creative attractor basins
- `LiminalityEngine` - Threshold states (4 forward-only phases)
- `InterSpeciesProtocol` - Cross-system relational interactions

#### Structural Module (Primitives 17-21)
- `ResonanceAddressService` - Pattern-based addressing
- `FractalGovernanceEngine` - Scale-invariant governance
- `MorphogeneticFieldService` - Developmental potential fields
- `TimeCrystalService` - Rhythmic temporal patterns
- `MycelialComputationService` - Distributed fungal-inspired tasks

#### Cycle Engine
- 28-day metabolism cycle orchestration
- 9 phase handlers with lifecycle hooks
- Phase-specific operation permissions
- Simulated time for testing

#### Holochain Zomes
- `living-metabolism` - Metabolism primitives
- `living-consciousness` - Consciousness primitives
- `living-epistemics` - Epistemics primitives
- `living-relational` - Relational primitives
- `living-structural` - Structural primitives
- `bridge` - v5.2 integration bridge
- `shared` - Common types

#### Solidity Contracts
- `WoundEscrow.sol` - Healing-oriented escrow
- `KenosisBurn.sol` - Reputation burning
- `FractalDAO.sol` - Scale-invariant governance

#### TypeScript SDK
- Full client SDK for all modules
- Type definitions matching Rust types
- Holochain client integration

#### Documentation
- `README.md` - Quick start guide
- `INTEGRATION.md` - v5.2 integration architecture
- `docs/ARCHITECTURE.md` - Mermaid diagrams
- `STATUS_REPORT.md` - Project status

#### Testing
- 442 unit and integration tests
- Property-based fuzzing with proptest
- Criterion benchmarks

#### DevOps
- GitHub Actions CI/CD pipeline
- Release automation
- Foundry configuration for Solidity

### Security
- Gate 1: Hard invariants (blocking)
- Gate 2: Soft constraints (warning)
- Gate 3: Network health (advisory)

## [0.5.2] - Previous Version

See [mycelix-property](https://github.com/mycelix/mycelix-property) for v5.2 changelog.

---

[Unreleased]: https://github.com/mycelix/mycelix-v6-living/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/mycelix/mycelix-v6-living/releases/tag/v0.6.0
