/**
 * Module B: Consciousness Field SDK
 * Primitives [5]-[8]: Temporal K-Vector, Field Interference, Collective Dreaming, Emergent Personhood
 */

import type { Did, KVectorSignature, ActionHash } from './types';

// =============================================================================
// Temporal K-Vector [5]
// =============================================================================

export interface TemporalKVectorSnapshot {
  agentDid: Did;
  kVector: KVectorSignature;
  velocity: number[];
  acceleration: number[];
  rateOfChange: number;
  anomalousChange: boolean;
  timestamp: string;
}

export interface KVectorSnapshotInput {
  kVector: KVectorSignature;
}

// =============================================================================
// Field Interference [6]
// =============================================================================

export enum InterferenceType {
  Constructive = 'Constructive',
  Destructive = 'Destructive',
  Mixed = 'Mixed',
}

export interface InterferenceDimension {
  dim: number;
  phaseDifference: number;
  amplitude: number;
  constructive: boolean;
}

export interface FieldInterferenceResult {
  agents: Did[];
  pattern: InterferenceDimension[];
  overallType: InterferenceType;
  amplitude: number;
  computedAt: string;
}

export interface GroupInterference {
  agents: Did[];
  pairwiseResults: FieldInterferenceResult[];
  dominantType: InterferenceType;
  meanAmplitude: number;
}

// =============================================================================
// Collective Dreaming [7]
// =============================================================================

export enum DreamState {
  Waking = 'Waking',
  Rem = 'Rem',
  Deep = 'Deep',
  Lucid = 'Lucid',
}

export interface DreamProposal {
  id: string;
  dreamState: DreamState;
  content: string;
  generatedAt: string;
  confirmed: boolean;
  confirmationThreshold: number;
  financialOperations: boolean; // Must always be false
}

export interface DreamProposalInput {
  content: string;
}

export interface DreamVoteInput {
  proposalHash: ActionHash;
  vote: boolean;
}

// =============================================================================
// Emergent Personhood [8]
// =============================================================================

export interface NetworkPhiMeasurement {
  phi: number;
  nodeCount: number;
  integrationScore: number;
  aggregateKVector: KVectorSignature;
  stdDev: number[];
  spectralK: number;
  computedAt: string;
}

// =============================================================================
// Client Functions
// =============================================================================

export interface ConsciousnessClient {
  // Temporal K-Vector
  submitKVectorSnapshot(input: KVectorSnapshotInput): Promise<TemporalKVectorSnapshot>;
  getKVectorHistory(agentDid: Did, limit?: number): Promise<TemporalKVectorSnapshot[]>;
  detectAnomalies(threshold: number): Promise<[Did, number[]][]>;

  // Field Interference
  computePairwiseInterference(agentA: Did, agentB: Did): Promise<FieldInterferenceResult>;
  computeGroupInterference(agents: Did[]): Promise<GroupInterference>;
  findConstructivePairs(threshold?: number): Promise<[Did, Did, number][]>;

  // Collective Dreaming
  getCurrentDreamState(): Promise<DreamState>;
  submitDreamProposal(input: DreamProposalInput): Promise<DreamProposal>;
  confirmDreamProposal(input: DreamVoteInput): Promise<boolean>;
  isFinancialBlocked(): Promise<boolean>;

  // Emergent Personhood
  computeNetworkPhi(): Promise<NetworkPhiMeasurement>;
  isNetworkConscious(phiThreshold?: number): Promise<boolean>;
}
