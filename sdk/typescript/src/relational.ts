/**
 * Module D: Relational Field SDK
 * Primitives [13]-[16]: Entangled Pairs, Eros/Attractor, Liminality, Inter-Species
 */

import type { Did, ActionHash, KVectorSignature } from './types';

// =============================================================================
// Entangled Pairs [13]
// =============================================================================

export interface EntangledPair {
  id: string;
  agentA: Did;
  agentB: Did;
  entanglementStrength: number;
  formed: string;
  lastCoCreation: string;
  decayRate: number;
}

export interface CoCreationEvent {
  agentA: Did;
  agentB: Did;
  description: string;
  qualityScore: number;
  timestamp: string;
}

export interface RecordCoCreationInput {
  partnerDid: Did;
  description: string;
  qualityScore: number;
}

export interface FormEntanglementInput {
  partnerDid: Did;
}

// =============================================================================
// Eros / Attractor Fields [14]
// =============================================================================

export interface AttractorField {
  centerDid: Did;
  attractedAgents: [Did, number][];
  fieldStrength: number;
  computedAt: string;
}

// =============================================================================
// Liminality [15]
// =============================================================================

export enum LiminalPhase {
  PreLiminal = 'PreLiminal',
  Liminal = 'Liminal',
  PostLiminal = 'PostLiminal',
  Integrated = 'Integrated',
}

export enum LiminalEntityType {
  Agent = 'Agent',
  Dao = 'Dao',
  Protocol = 'Protocol',
  Community = 'Community',
}

export interface LiminalRecord {
  id: string;
  entityDid: Did;
  entityType: LiminalEntityType;
  phase: LiminalPhase;
  entered: string;
  previousIdentity?: string;
  emergingIdentity?: string;
  recategorizationBlocked: boolean;
}

export interface EnterLiminalInput {
  entityDid: Did;
  entityType: LiminalEntityType;
  previousIdentity?: string;
}

// =============================================================================
// Inter-Species Participation [16]
// =============================================================================

export enum SpeciesType {
  Human = 'Human',
  AiAgent = 'AiAgent',
  Dao = 'Dao',
  Sensor = 'Sensor',
  Ecological = 'Ecological',
  Other = 'Other',
}

export interface InterSpeciesParticipant {
  id: string;
  species: SpeciesType;
  bridgeProtocol: string;
  capabilities: string[];
  constraints: string[];
  registered: string;
}

export interface RegisterInterSpeciesInput {
  species: SpeciesType;
  bridgeProtocol: string;
  capabilities: string[];
  constraints: string[];
}

// =============================================================================
// Client Functions
// =============================================================================

export interface RelationalClient {
  // Entangled Pairs
  recordCoCreation(input: RecordCoCreationInput): Promise<CoCreationEvent>;
  formEntanglement(input: FormEntanglementInput): Promise<EntangledPair>;
  getEntangledPartners(agentDid: Did): Promise<[Did, number][]>;
  getEntanglementStrength(agentA: Did, agentB: Did): Promise<number>;

  // Eros / Attractor Fields
  computeAttractorFields(): Promise<AttractorField[]>;
  findComplementaryAgents(agentDid: Did, topN?: number): Promise<[Did, number][]>;

  // Liminality
  enterLiminalState(input: EnterLiminalInput): Promise<LiminalRecord>;
  advanceLiminalPhase(recordHash: ActionHash): Promise<LiminalRecord>;
  isInLiminalState(entityDid: Did): Promise<boolean>;
  getLiminalEntities(): Promise<LiminalRecord[]>;

  // Inter-Species
  registerInterSpecies(input: RegisterInterSpeciesInput): Promise<InterSpeciesParticipant>;
  getParticipantsBySpecies(species: SpeciesType): Promise<InterSpeciesParticipant[]>;
  canParticipate(participantId: string, action: string): Promise<boolean>;
}
