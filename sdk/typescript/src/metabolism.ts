/**
 * Module A: Metabolism Engine SDK
 * Primitives [1]-[4]: Composting, Wound Healing, Metabolic Trust, Kenosis
 */

import type { Did, ActionHash, EpistemicClassification } from './types';

// =============================================================================
// Wound Healing [2]
// =============================================================================

export enum WoundPhase {
  Hemostasis = 'Hemostasis',
  Inflammation = 'Inflammation',
  Proliferation = 'Proliferation',
  Remodeling = 'Remodeling',
  Healed = 'Healed',
}

export enum WoundSeverity {
  Minor = 'Minor',
  Moderate = 'Moderate',
  Severe = 'Severe',
  Critical = 'Critical',
}

export interface WoundRecord {
  id: string;
  agentDid: Did;
  severity: WoundSeverity;
  cause: string;
  phase: WoundPhase;
  created: string;
  phaseHistory: [WoundPhase, string][];
  restitutionRequired?: RestitutionRequirement;
  scarTissue?: ScarTissue;
}

export interface RestitutionRequirement {
  description: string;
  amountFlow?: number;
  actionsRequired: string[];
  deadline: string;
  fulfilled: boolean;
}

export interface ScarTissue {
  area: string;
  strengthMultiplier: number;
  formed: string;
}

export interface CreateWoundInput {
  agentDid: Did;
  severity: WoundSeverity;
  cause: string;
}

// =============================================================================
// Metabolic Trust [3]
// =============================================================================

export interface MetabolicTrustScore {
  matlComposite: number;
  throughput: number;
  resilience: number;
  compostingContribution: number;
  metabolicTrust: number;
  computedAt: string;
}

export interface UpdateTrustInput {
  agentDid: Did;
  matlComposite: number;
  throughput: number;
  resilience: number;
  compostingContribution: number;
}

// =============================================================================
// Kenosis [4]
// =============================================================================

export interface KenosisCommitment {
  id: string;
  agentDid: Did;
  releasePercentage: number;
  reputationReleased: number;
  committedAt: string;
  cycleNumber: number;
  irrevocable: boolean;
}

export interface CommitKenosisInput {
  agentDid: Did;
  releasePercentage: number;
}

// =============================================================================
// Composting [1]
// =============================================================================

export enum CompostableEntity {
  FailedProposal = 'FailedProposal',
  AbandonedProject = 'AbandonedProject',
  ExpiredClaim = 'ExpiredClaim',
  DeprecatedComponent = 'DeprecatedComponent',
  DissolvedDao = 'DissolvedDao',
}

export interface CompostingRecord {
  id: string;
  entityType: CompostableEntity;
  entityId: string;
  started: string;
  nutrients: Nutrient[];
  decompositionProgress: number;
  completed?: string;
}

export interface Nutrient {
  id: string;
  sourceEntity: string;
  learning: string;
  classification: EpistemicClassification;
  extractedAt: string;
  published: boolean;
}

export interface StartCompostingInput {
  entityType: CompostableEntity;
  entityId: string;
  reason: string;
}

// =============================================================================
// Client Functions
// =============================================================================

export interface MetabolismClient {
  // Wound Healing
  createWound(input: CreateWoundInput): Promise<WoundRecord>;
  advanceWoundPhase(woundHash: ActionHash): Promise<WoundRecord>;
  getWoundsForAgent(agentDid: Did): Promise<WoundRecord[]>;

  // Metabolic Trust
  updateMetabolicTrust(input: UpdateTrustInput): Promise<MetabolicTrustScore>;
  getMetabolicTrust(agentDid: Did): Promise<MetabolicTrustScore | null>;

  // Kenosis
  commitKenosis(input: CommitKenosisInput): Promise<KenosisCommitment>;
  getCycleReleases(agentDid: Did, cycleNumber: number): Promise<number>;

  // Composting
  startComposting(input: StartCompostingInput): Promise<CompostingRecord>;
  extractNutrient(recordId: string, learning: string, classification: EpistemicClassification): Promise<Nutrient>;
  completeComposting(recordId: string): Promise<Nutrient[]>;
  getActiveComposting(): Promise<CompostingRecord[]>;
}
