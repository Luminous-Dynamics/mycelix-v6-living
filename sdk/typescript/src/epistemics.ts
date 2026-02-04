/**
 * Module C: Epistemic Deepening SDK
 * Primitives [9]-[12]: Shadow Integration, Negative Capability, Silence as Signal, Beauty as Validity
 */

import type { Did, ClaimId, ActionHash, EpistemicClassification } from './types';

// =============================================================================
// Shadow Integration [9]
// =============================================================================

export interface ShadowRecord {
  id: string;
  originalContentId: string;
  suppressedAt: string;
  surfacedAt: string;
  suppressionReason: string;
  lowRepDissent: boolean;
  spectralKAnomaly?: number;
}

export interface SurfaceShadowInput {
  contentId: string;
  reason: string;
}

// =============================================================================
// Negative Capability [10]
// =============================================================================

export interface HeldInUncertaintyClaim {
  claimId: ClaimId;
  reason: string;
  heldSince: string;
  earliestResolution: string;
  heldBy: Did;
}

export interface HoldInUncertaintyInput {
  claimId: ClaimId;
  reason: string;
  minHoldDays: number;
}

export interface ReleaseFromUncertaintyInput {
  claimHash: ActionHash;
  resolution: string;
}

// =============================================================================
// Silence as Signal [11]
// =============================================================================

export enum SilenceClassification {
  DeliberateWithholding = 'DeliberateWithholding',
  Contemplative = 'Contemplative',
  DissentThroughAbsence = 'DissentThroughAbsence',
  Unknown = 'Unknown',
}

export enum PresenceStatus {
  Active = 'Active',
  Silent = 'Silent',
  Absent = 'Absent',
}

export interface PresenceProof {
  agentDid: Did;
  timestamp: string;
  heartbeatHash: Uint8Array;
  signature: Uint8Array;
}

export interface SilenceRecord {
  agentDid: Did;
  topic: string;
  silenceStarted: string;
  presenceProofs: PresenceProof[];
  classification: SilenceClassification;
}

export interface RecordSilenceInput {
  agentDid: Did;
  topic: string;
  presenceProofs: PresenceProof[];
}

// =============================================================================
// Beauty as Validity [12]
// =============================================================================

export interface BeautyScore {
  symmetry: number;
  economy: number;
  resonance: number;
  surprise: number;
  completeness: number;
  composite: number;
}

export interface BeautyScoreRecord {
  proposalId: string;
  score: BeautyScore;
  scorerDid: Did;
  timestamp: string;
}

export interface SubmitBeautyScoreInput {
  proposalId: string;
  proposalContent: string;
  existingPatterns?: string[];
  requirements?: string[];
}

// =============================================================================
// Client Functions
// =============================================================================

export interface EpistemicsClient {
  // Shadow Integration
  surfaceShadow(input: SurfaceShadowInput): Promise<ShadowRecord>;
  getSurfacedShadows(limit?: number): Promise<ShadowRecord[]>;

  // Negative Capability
  holdInUncertainty(input: HoldInUncertaintyInput): Promise<HeldInUncertaintyClaim>;
  releaseFromUncertainty(input: ReleaseFromUncertaintyInput): Promise<void>;
  isClaimHeld(claimId: ClaimId): Promise<boolean>;
  canVoteOn(claimId: ClaimId): Promise<boolean>;
  getHeldClaims(): Promise<HeldInUncertaintyClaim[]>;

  // Silence as Signal
  submitHeartbeat(proof: PresenceProof): Promise<boolean>;
  detectSilences(topic: string, minDurationHours?: number): Promise<SilenceRecord[]>;
  getPresenceStatus(agentDid: Did): Promise<PresenceStatus>;

  // Beauty as Validity
  submitBeautyScore(input: SubmitBeautyScoreInput): Promise<BeautyScoreRecord>;
  getBeautyScores(proposalId: string): Promise<BeautyScoreRecord[]>;
  getAggregateBeautyScore(proposalId: string): Promise<BeautyScore | null>;
  meetsBeautyThreshold(proposalId: string, threshold?: number): Promise<boolean>;
}
