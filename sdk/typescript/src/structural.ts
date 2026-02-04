/**
 * Module E: Structural Emergence SDK
 * Primitives [17]-[21]: Resonance Addressing, Fractal Governance, Morphogenetic Fields,
 *                       Time-Crystal Consensus, Mycelial Computation
 */

import type { Did, ActionHash } from './types';

// =============================================================================
// Resonance Addressing [17]
// =============================================================================

export interface ResonanceAddress {
  patternHash: Uint8Array;
  semanticEmbedding: number[];
  harmonicSignature: number[];
  created: string;
}

export interface ResonanceAddressEntry {
  address: ResonanceAddress;
  ownerDid: Did;
  contentId: string;
}

export interface CreateResonanceAddressInput {
  content: string;
  ownerDid: Did;
}

export interface ResolveByPatternInput {
  pattern: number[];
  threshold: number;
}

// =============================================================================
// Fractal Governance [18]
// =============================================================================

export enum GovernanceScale {
  Individual = 'Individual',
  Team = 'Team',
  Community = 'Community',
  Sector = 'Sector',
  Regional = 'Regional',
  Global = 'Global',
}

export enum DecisionMechanism {
  Consent = 'Consent',
  Consensus = 'Consensus',
  Supermajority = 'Supermajority',
  ReputationWeighted = 'ReputationWeighted',
}

export interface FractalGovernancePattern {
  id: string;
  scale: GovernanceScale;
  parentPatternId?: string;
  childPatterns: string[];
  quorumRatio: number;
  supermajorityRatio: number;
  decisionMechanism: DecisionMechanism;
}

export interface CreateFractalPatternInput {
  scale: GovernanceScale;
  quorumRatio: number;
  supermajorityRatio: number;
  decisionMechanism: DecisionMechanism;
}

// =============================================================================
// Morphogenetic Fields [19]
// =============================================================================

export enum FieldType {
  Attracting = 'Attracting',
  Repelling = 'Repelling',
  Guiding = 'Guiding',
  Containing = 'Containing',
}

export interface MorphogeneticField {
  id: string;
  fieldType: FieldType;
  strength: number;
  gradient: number[];
  sourcePatternId: string;
  created: string;
}

export interface CreateMorphogeneticFieldInput {
  fieldType: FieldType;
  sourcePatternId: string;
  initialStrength: number;
}

// =============================================================================
// Time-Crystal Consensus [20]
// =============================================================================

export interface TimeCrystalPeriod {
  periodId: number;
  phaseAngle: number;
  symmetryGroup: string;
  validators: Did[];
  started: string;
  periodDuration: number; // milliseconds
}

export interface StartTimeCrystalInput {
  validators: Did[];
  periodDurationMs: number;
}

// =============================================================================
// Mycelial Computation [21]
// =============================================================================

export interface MycelialTask {
  id: string;
  computation: string;
  inputHash: Uint8Array;
  assignedNodes: Did[];
  resultHash?: Uint8Array;
  started: string;
  completed?: string;
}

export enum AssignmentStrategy {
  NearestNeighbor = 'NearestNeighbor',
  LoadBalanced = 'LoadBalanced',
  CapabilityMatched = 'CapabilityMatched',
}

export interface SubmitMycelialTaskInput {
  computation: string;
  inputHash: Uint8Array;
  strategy: AssignmentStrategy;
}

// =============================================================================
// Client Functions
// =============================================================================

export interface StructuralClient {
  // Resonance Addressing
  createResonanceAddress(input: CreateResonanceAddressInput): Promise<ResonanceAddressEntry>;
  resolveByPattern(input: ResolveByPatternInput): Promise<ResonanceAddressEntry[]>;
  resolveByHash(hash: Uint8Array): Promise<ResonanceAddressEntry | null>;

  // Fractal Governance
  createFractalPattern(input: CreateFractalPatternInput): Promise<FractalGovernancePattern>;
  replicatePattern(parentHash: ActionHash, childScale: GovernanceScale): Promise<FractalGovernancePattern>;
  verifyStructuralIdentity(patternA: string, patternB: string): Promise<boolean>;
  getPatternsAtScale(scale: GovernanceScale): Promise<FractalGovernancePattern[]>;

  // Morphogenetic Fields
  createField(input: CreateMorphogeneticFieldInput): Promise<MorphogeneticField>;
  updateFieldStrength(fieldId: string, delta: number): Promise<MorphogeneticField>;
  computeGradient(fieldId: string, position: number[]): Promise<number[]>;
  getActiveFields(): Promise<MorphogeneticField[]>;

  // Time-Crystal Consensus
  startPeriod(input: StartTimeCrystalInput): Promise<TimeCrystalPeriod>;
  getCurrentPeriod(): Promise<TimeCrystalPeriod | null>;
  getValidatorForPhase(phaseAngle: number): Promise<Did | null>;

  // Mycelial Computation
  submitTask(input: SubmitMycelialTaskInput): Promise<MycelialTask>;
  getTaskStatus(taskId: string): Promise<MycelialTask>;
  getPendingTasks(): Promise<MycelialTask[]>;
}
