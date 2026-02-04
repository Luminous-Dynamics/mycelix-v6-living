/**
 * Metabolism Cycle Engine SDK
 * Orchestrator for the 28-day lunar metabolism cycle.
 */

import type {
  CyclePhase,
  CycleState,
  PhaseTransition,
  PhaseMetrics,
  PHASE_ORDER,
  PHASE_DURATIONS,
} from './types';

// =============================================================================
// Cycle Event Types
// =============================================================================

export type LivingProtocolEvent =
  // Metabolism
  | { type: 'CompostingStarted'; data: { recordId: string; entityType: string; entityId: string } }
  | { type: 'NutrientExtracted'; data: { recordId: string; learning: string } }
  | { type: 'CompostingCompleted'; data: { recordId: string; totalNutrients: number } }
  | { type: 'WoundCreated'; data: { woundId: string; agentDid: string; severity: string } }
  | { type: 'WoundPhaseAdvanced'; data: { woundId: string; from: string; to: string } }
  | { type: 'KenosisCommitted'; data: { commitmentId: string; releasePercentage: number } }
  | { type: 'MetabolicTrustUpdated'; data: { agentDid: string; newScore: number } }
  // Consciousness
  | { type: 'TemporalKVectorUpdated'; data: { agentDid: string; rateOfChange: number } }
  | { type: 'FieldInterferenceDetected'; data: { agents: string[]; interferenceType: string } }
  | { type: 'DreamStateChanged'; data: { from: string; to: string } }
  | { type: 'NetworkPhiComputed'; data: { phi: number; nodeCount: number } }
  // Epistemics
  | { type: 'ShadowSurfaced'; data: { contentId: string; reason: string } }
  | { type: 'ClaimHeldInUncertainty'; data: { claimId: string; reason: string } }
  | { type: 'ClaimReleasedFromUncertainty'; data: { claimId: string; resolution: string } }
  | { type: 'SilenceDetected'; data: { agentDid: string; topic: string; classification: string } }
  | { type: 'BeautyScored'; data: { proposalId: string; composite: number } }
  // Relational
  | { type: 'EntanglementFormed'; data: { agentA: string; agentB: string; strength: number } }
  | { type: 'EntanglementDecayed'; data: { pairId: string; finalStrength: number } }
  | { type: 'AttractorFieldComputed'; data: { centerDid: string; fieldStrength: number } }
  | { type: 'LiminalTransitionStarted'; data: { entityDid: string; entityType: string } }
  | { type: 'LiminalTransitionCompleted'; data: { entityDid: string; newIdentity?: string } }
  // Structural
  | { type: 'ResonanceAddressCreated'; data: { ownerDid: string } }
  | { type: 'FractalPatternReplicated'; data: { parentScale: string; childScale: string } }
  | { type: 'TimeCrystalPeriodStarted'; data: { periodId: number } }
  | { type: 'MycelialTaskDistributed'; data: { taskId: string } }
  | { type: 'MycelialTaskCompleted'; data: { taskId: string } }
  // Cycle
  | { type: 'PhaseTransitioned'; data: PhaseTransition }
  | { type: 'CycleStarted'; data: { cycleNumber: number; startedAt: string } };

// =============================================================================
// Cycle Client
// =============================================================================

export interface CycleClient {
  /** Get current cycle state */
  getCurrentState(): Promise<CycleState>;

  /** Get current phase */
  getCurrentPhase(): Promise<CyclePhase>;

  /** Get current cycle number */
  getCycleNumber(): Promise<number>;

  /** Check if an operation is permitted in the current phase */
  isOperationPermitted(operation: string): Promise<boolean>;

  /** Check if financial operations are blocked */
  isFinancialBlocked(): Promise<boolean>;

  /** Get time remaining in current phase (milliseconds) */
  getTimeRemaining(): Promise<number>;

  /** Get phase transition history */
  getTransitionHistory(): Promise<PhaseTransition[]>;

  /** Subscribe to cycle events */
  onEvent(callback: (event: LivingProtocolEvent) => void): () => void;

  /** Get phase metrics */
  getPhaseMetrics(phase: CyclePhase): Promise<PhaseMetrics>;
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Get the next phase in the cycle.
 */
export function getNextPhase(current: CyclePhase): CyclePhase {
  const { PHASE_ORDER } = require('./types');
  const idx = PHASE_ORDER.indexOf(current);
  return PHASE_ORDER[(idx + 1) % PHASE_ORDER.length];
}

/**
 * Get the previous phase in the cycle.
 */
export function getPreviousPhase(current: CyclePhase): CyclePhase {
  const { PHASE_ORDER } = require('./types');
  const idx = PHASE_ORDER.indexOf(current);
  return PHASE_ORDER[(idx - 1 + PHASE_ORDER.length) % PHASE_ORDER.length];
}

/**
 * Check if voting is blocked in the given phase.
 */
export function isVotingBlocked(phase: CyclePhase): boolean {
  const { CyclePhase: CP } = require('./types');
  return phase === CP.NegativeCapability;
}

/**
 * Check if Gate 2 warnings are suspended in the given phase.
 */
export function isGate2Suspended(phase: CyclePhase): boolean {
  const { CyclePhase: CP } = require('./types');
  return phase === CP.Shadow;
}

/**
 * Calculate total cycle length in days.
 */
export function getTotalCycleDays(): number {
  const { PHASE_DURATIONS } = require('./types');
  return (Object.values(PHASE_DURATIONS) as number[]).reduce((sum: number, d: number) => sum + d, 0);
}
