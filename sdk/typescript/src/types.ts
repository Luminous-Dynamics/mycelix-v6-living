/**
 * Core types shared across all Living Protocol SDK modules.
 */

// =============================================================================
// Identity Types
// =============================================================================

export type Did = string;
export type ClaimId = string;
export type EntityId = string;
export type ActionHash = Uint8Array;
export type AgentPubKey = Uint8Array;
export type HashDigest = Uint8Array;

// =============================================================================
// Epistemic Classification
// =============================================================================

export enum EpistemicTier {
  Null = 0,
  Testimonial = 1,
  PrivatelyVerifiable = 2,
  CryptographicallyProven = 3,
  FormallyVerified = 4,
}

export enum NormativeTier {
  Personal = 0,
  Communal = 1,
  NetworkConsensus = 2,
  Axiomatic = 3,
}

export enum MaterialityTier {
  Ephemeral = 0,
  Temporal = 1,
  Persistent = 2,
  Foundational = 3,
}

export interface EpistemicClassification {
  e: EpistemicTier;
  n: NormativeTier;
  m: MaterialityTier;
}

// =============================================================================
// Claim Status
// =============================================================================

export type ClaimStatus =
  | { type: 'Active' }
  | { type: 'Disputed' }
  | { type: 'Resolved' }
  | { type: 'Refuted' }
  | { type: 'Superseded' }
  | {
      type: 'HeldInUncertainty';
      reason: string;
      heldSince: string;
      earliestResolution: string;
    }
  | {
      type: 'Composting';
      started: string;
      nutrientsExtracted: string[];
    }
  | {
      type: 'InShadow';
      surfaced: string;
      originalSuppression: string;
    };

// =============================================================================
// Cycle Types
// =============================================================================

export enum CyclePhase {
  Shadow = 'Shadow',
  Composting = 'Composting',
  Liminal = 'Liminal',
  NegativeCapability = 'NegativeCapability',
  Eros = 'Eros',
  CoCreation = 'CoCreation',
  Beauty = 'Beauty',
  EmergentPersonhood = 'EmergentPersonhood',
  Kenosis = 'Kenosis',
}

export const PHASE_DURATIONS: Record<CyclePhase, number> = {
  [CyclePhase.Shadow]: 2,
  [CyclePhase.Composting]: 5,
  [CyclePhase.Liminal]: 3,
  [CyclePhase.NegativeCapability]: 3,
  [CyclePhase.Eros]: 4,
  [CyclePhase.CoCreation]: 7,
  [CyclePhase.Beauty]: 2,
  [CyclePhase.EmergentPersonhood]: 1,
  [CyclePhase.Kenosis]: 1,
};

export const PHASE_ORDER: CyclePhase[] = [
  CyclePhase.Shadow,
  CyclePhase.Composting,
  CyclePhase.Liminal,
  CyclePhase.NegativeCapability,
  CyclePhase.Eros,
  CyclePhase.CoCreation,
  CyclePhase.Beauty,
  CyclePhase.EmergentPersonhood,
  CyclePhase.Kenosis,
];

export interface CycleState {
  cycleNumber: number;
  currentPhase: CyclePhase;
  phaseStarted: string;
  cycleStarted: string;
  phaseDay: number;
}

export interface PhaseMetrics {
  activeAgents: number;
  spectralK: number;
  meanMetabolicTrust: number;
  activeWounds: number;
  compostingEntities: number;
  liminalEntities: number;
  entangledPairs: number;
  heldUncertainties: number;
}

export interface PhaseTransition {
  from: CyclePhase;
  to: CyclePhase;
  cycleNumber: number;
  transitionedAt: string;
  metrics: PhaseMetrics;
}

// =============================================================================
// K-Vector Types
// =============================================================================

export interface KVectorSignature {
  kR: number;
  kA: number;
  kI: number;
  kP: number;
  kM: number;
  kS: number;
  kH: number;
  kTopo: number;
  timestamp: string;
  signature: Uint8Array;
}

export interface TemporalKVector {
  current: KVectorSignature;
  velocity: number[];
  acceleration: number[];
  computedAt: string;
}

// =============================================================================
// Three Gates Types
// =============================================================================

export interface Gate1Check {
  invariant: string;
  passed: boolean;
  details?: string;
}

export interface Gate2Warning {
  harmonyViolated: string;
  severity: number;
  reputationImpact: number;
  reasoning: string;
  userMayProceed: boolean;
}

export interface Gate3Consequence {
  action: string;
  attestations: [Did, number][];
  reputationDelta: number;
}

// =============================================================================
// Feature Flags
// =============================================================================

export interface FeatureFlags {
  // Tier 1
  metabolicTrust: boolean;
  temporalKVector: boolean;
  negativeCapability: boolean;
  silenceAsSignal: boolean;
  beautyAsValidity: boolean;
  liminality: boolean;
  // Tier 2
  composting: boolean;
  woundHealing: boolean;
  kenosis: boolean;
  shadowIntegration: boolean;
  entangledPairs: boolean;
  resonanceAddressing: boolean;
  fractalGovernance: boolean;
  morphogeneticFields: boolean;
  // Tier 3
  fieldInterference: boolean;
  collectiveDreaming: boolean;
  erosAttractor: boolean;
  timeCrystal: boolean;
  mycelialComputation: boolean;
  // Tier 4
  emergentPersonhood: boolean;
  interSpecies: boolean;
}
