/**
 * Subscription Manager
 * Provides filtered subscriptions to Living Protocol events.
 */

import { CyclePhase } from './types';
import { LivingProtocolEvent } from './cycle';

// =============================================================================
// Filter Types
// =============================================================================

export interface SubscriptionFilter {
  /** Only receive events of these types */
  eventTypes?: LivingProtocolEvent['type'][];

  /** Only receive events during these phases */
  phases?: CyclePhase[];

  /** Only receive events involving these agent DIDs */
  agentDids?: string[];

  /** Custom filter function */
  customFilter?: (event: LivingProtocolEvent) => boolean;
}

export type EventCallback = (event: LivingProtocolEvent) => void;
export type Unsubscribe = () => void;

// =============================================================================
// Subscription Manager
// =============================================================================

export class SubscriptionManager {
  private subscriptions: Map<
    number,
    { filter: SubscriptionFilter; callback: EventCallback }
  > = new Map();
  private nextSubscriptionId = 0;
  private currentPhase: CyclePhase = CyclePhase.Shadow;

  /**
   * Subscribe to events matching the given filter.
   *
   * @param filter Filter to apply to events
   * @param callback Function to call for matching events
   * @returns Unsubscribe function
   */
  subscribe(filter: SubscriptionFilter, callback: EventCallback): Unsubscribe {
    const id = this.nextSubscriptionId++;
    this.subscriptions.set(id, { filter, callback });

    return () => {
      this.subscriptions.delete(id);
    };
  }

  /**
   * Subscribe to all events (no filter).
   */
  subscribeAll(callback: EventCallback): Unsubscribe {
    return this.subscribe({}, callback);
  }

  /**
   * Subscribe to specific event types.
   */
  subscribeToTypes(
    types: LivingProtocolEvent['type'][],
    callback: EventCallback
  ): Unsubscribe {
    return this.subscribe({ eventTypes: types }, callback);
  }

  /**
   * Subscribe to events for specific agents.
   */
  subscribeToAgents(
    agentDids: string[],
    callback: EventCallback
  ): Unsubscribe {
    return this.subscribe({ agentDids }, callback);
  }

  /**
   * Subscribe to events during specific phases.
   */
  subscribeToPhases(
    phases: CyclePhase[],
    callback: EventCallback
  ): Unsubscribe {
    return this.subscribe({ phases }, callback);
  }

  /**
   * Update the current phase (used for phase filtering).
   */
  setCurrentPhase(phase: CyclePhase): void {
    this.currentPhase = phase;
  }

  /**
   * Dispatch an event to matching subscribers.
   */
  dispatch(event: LivingProtocolEvent): void {
    // Update phase if this is a phase transition event
    if (event.type === 'PhaseTransitioned') {
      this.currentPhase = event.data.to;
    }

    this.subscriptions.forEach(({ filter, callback }) => {
      if (this.matchesFilter(event, filter)) {
        try {
          callback(event);
        } catch (error) {
          console.error('Subscription callback error:', error);
        }
      }
    });
  }

  /**
   * Get the number of active subscriptions.
   */
  getSubscriptionCount(): number {
    return this.subscriptions.size;
  }

  /**
   * Clear all subscriptions.
   */
  clearAll(): void {
    this.subscriptions.clear();
  }

  // ---------------------------------------------------------------------------
  // Filter Matching
  // ---------------------------------------------------------------------------

  private matchesFilter(event: LivingProtocolEvent, filter: SubscriptionFilter): boolean {
    // Event type filter
    if (filter.eventTypes && filter.eventTypes.length > 0) {
      if (!filter.eventTypes.includes(event.type)) {
        return false;
      }
    }

    // Phase filter
    if (filter.phases && filter.phases.length > 0) {
      if (!filter.phases.includes(this.currentPhase)) {
        return false;
      }
    }

    // Agent DID filter
    if (filter.agentDids && filter.agentDids.length > 0) {
      if (!this.eventInvolvesAgent(event, filter.agentDids)) {
        return false;
      }
    }

    // Custom filter
    if (filter.customFilter) {
      if (!filter.customFilter(event)) {
        return false;
      }
    }

    return true;
  }

  private eventInvolvesAgent(event: LivingProtocolEvent, agentDids: string[]): boolean {
    // Extract agent DIDs from event data based on event type
    const data = (event as { data?: Record<string, unknown> }).data;

    if (!data) return false;

    // Check common agent fields
    const agentFields = ['agentDid', 'agentA', 'agentB', 'entityDid', 'ownerDid', 'centerDid'];
    for (const field of agentFields) {
      if (data[field] && agentDids.includes(data[field] as string)) {
        return true;
      }
    }

    // Check agent arrays
    if (data.agents && Array.isArray(data.agents)) {
      for (const agent of data.agents) {
        if (agentDids.includes(agent as string)) {
          return true;
        }
      }
    }

    return false;
  }
}

// =============================================================================
// Pre-built Subscription Filters
// =============================================================================

/**
 * Filter for metabolism events (wounds, kenosis, trust).
 */
export const METABOLISM_FILTER: SubscriptionFilter = {
  eventTypes: [
    'WoundCreated',
    'WoundPhaseAdvanced',
    'KenosisCommitted',
    'MetabolicTrustUpdated',
    'CompostingStarted',
    'NutrientExtracted',
    'CompostingCompleted',
  ],
};

/**
 * Filter for consciousness events (k-vectors, phi, dreams).
 */
export const CONSCIOUSNESS_FILTER: SubscriptionFilter = {
  eventTypes: [
    'TemporalKVectorUpdated',
    'FieldInterferenceDetected',
    'DreamStateChanged',
    'NetworkPhiComputed',
  ],
};

/**
 * Filter for epistemics events (shadows, uncertainty, silence, beauty).
 */
export const EPISTEMICS_FILTER: SubscriptionFilter = {
  eventTypes: [
    'ShadowSurfaced',
    'ClaimHeldInUncertainty',
    'ClaimReleasedFromUncertainty',
    'SilenceDetected',
    'BeautyScored',
  ],
};

/**
 * Filter for relational events (entanglement, liminal transitions).
 */
export const RELATIONAL_FILTER: SubscriptionFilter = {
  eventTypes: [
    'EntanglementFormed',
    'EntanglementDecayed',
    'AttractorFieldComputed',
    'LiminalTransitionStarted',
    'LiminalTransitionCompleted',
  ],
};

/**
 * Filter for structural events (resonance, fractal, mycelial).
 */
export const STRUCTURAL_FILTER: SubscriptionFilter = {
  eventTypes: [
    'ResonanceAddressCreated',
    'FractalPatternReplicated',
    'TimeCrystalPeriodStarted',
    'MycelialTaskDistributed',
    'MycelialTaskCompleted',
  ],
};

/**
 * Filter for cycle events (phase transitions, cycle starts).
 */
export const CYCLE_FILTER: SubscriptionFilter = {
  eventTypes: ['PhaseTransitioned', 'CycleStarted'],
};
