/**
 * Living Protocol Client
 * Full-featured client with WebSocket subscriptions and type-safe API.
 */

import { CyclePhase, CycleState, PhaseTransition, PhaseMetrics, PHASE_ORDER } from './types';
import { CycleClient, LivingProtocolEvent, getNextPhase, isVotingBlocked } from './cycle';
import { WebSocketTransport, TransportConfig, TransportState } from './transport';
import { SubscriptionManager, SubscriptionFilter, EventCallback, Unsubscribe } from './subscription';

// =============================================================================
// Client Configuration
// =============================================================================

export interface LivingProtocolClientConfig extends TransportConfig {
  /** Whether to auto-connect on creation */
  autoConnect?: boolean;
}

export interface ConnectionOptions {
  /** Timeout for connection attempt (ms) */
  timeout?: number;
}

// =============================================================================
// Living Protocol Client
// =============================================================================

/**
 * Full-featured client for the Living Protocol.
 *
 * Provides:
 * - WebSocket-based real-time event subscriptions
 * - Type-safe cycle state queries
 * - Filtered event subscriptions
 *
 * @example
 * ```typescript
 * const client = await LivingProtocolClient.connect({
 *   url: 'ws://localhost:8888/ws'
 * });
 *
 * // Subscribe to phase transitions
 * client.onPhaseChange((event) => {
 *   console.log(`Phase changed to ${event.data.to}`);
 * });
 *
 * // Get current state
 * const state = await client.getCurrentState();
 * console.log(`Currently in ${state.currentPhase}, cycle ${state.cycleNumber}`);
 * ```
 */
// Request/response types for WebSocket RPC
interface PendingRequest<T> {
  resolve: (value: T) => void;
  reject: (error: Error) => void;
  timeout: NodeJS.Timeout;
}

interface RequestMessage {
  id: string;
  method: string;
  params?: unknown;
}

interface ResponseMessage {
  id: string;
  result?: unknown;
  error?: { code: number; message: string };
}

export class LivingProtocolClient implements CycleClient {
  private transport: WebSocketTransport;
  private subscriptions: SubscriptionManager;
  private cachedState: CycleState | null = null;
  private stateTimestamp: number = 0;
  private readonly stateCacheTtl = 5000; // 5 seconds

  // Request/response tracking
  private requestId = 0;
  private pendingRequests: Map<string, PendingRequest<unknown>> = new Map();
  private readonly requestTimeoutMs = 30000; // 30 seconds

  private constructor(transport: WebSocketTransport) {
    this.transport = transport;
    this.subscriptions = new SubscriptionManager();

    // Wire transport events to subscription manager and request handling
    this.transport.onProtocolEvent((event) => {
      this.handleEvent(event);
    });

    // Handle response messages
    this.transport.on('message', (transportEvent) => {
      if (transportEvent.type === 'message') {
        this.handleResponse(transportEvent.data as ResponseMessage);
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Static Factory
  // ---------------------------------------------------------------------------

  /**
   * Connect to a Living Protocol server.
   *
   * @param config Connection configuration
   * @returns Connected client instance
   */
  static async connect(config: LivingProtocolClientConfig): Promise<LivingProtocolClient> {
    const transport = new WebSocketTransport(config);
    const client = new LivingProtocolClient(transport);

    if (config.autoConnect !== false) {
      await transport.connect();
    }

    return client;
  }

  // ---------------------------------------------------------------------------
  // Connection Management
  // ---------------------------------------------------------------------------

  /**
   * Connect to the server (if not already connected).
   */
  async connect(options?: ConnectionOptions): Promise<void> {
    await this.transport.connect();
  }

  /**
   * Disconnect from the server.
   */
  disconnect(): void {
    // Reject all pending requests
    for (const [id, pending] of this.pendingRequests) {
      clearTimeout(pending.timeout);
      pending.reject(new Error('Connection closed'));
    }
    this.pendingRequests.clear();

    this.transport.disconnect();
    this.subscriptions.clearAll();
  }

  /**
   * Check if connected.
   */
  isConnected(): boolean {
    return this.transport.isConnected();
  }

  /**
   * Get connection state.
   */
  getConnectionState(): TransportState {
    return this.transport.getState();
  }

  /**
   * Subscribe to connection state changes.
   */
  onConnectionStateChange(callback: (state: TransportState) => void): Unsubscribe {
    return this.transport.on('stateChange', (event) => {
      if (event.type === 'stateChange') {
        callback(event.state);
      }
    });
  }

  // ---------------------------------------------------------------------------
  // CycleClient Implementation
  // ---------------------------------------------------------------------------

  async getCurrentState(): Promise<CycleState> {
    // Return cached state if fresh enough
    if (this.cachedState && Date.now() - this.stateTimestamp < this.stateCacheTtl) {
      return this.cachedState;
    }

    // Request fresh state from server
    const state = await this.requestState();
    this.cachedState = state;
    this.stateTimestamp = Date.now();

    return state;
  }

  async getCurrentPhase(): Promise<CyclePhase> {
    const state = await this.getCurrentState();
    return state.currentPhase;
  }

  async getCycleNumber(): Promise<number> {
    const state = await this.getCurrentState();
    return state.cycleNumber;
  }

  async isOperationPermitted(operation: string): Promise<boolean> {
    const phase = await this.getCurrentPhase();

    switch (operation) {
      case 'vote':
        return !isVotingBlocked(phase);
      case 'kenosis':
        return phase === CyclePhase.Kenosis;
      case 'composting':
        return phase === CyclePhase.Composting;
      case 'beauty_score':
        return phase === CyclePhase.Beauty;
      case 'liminal_enter':
        return phase === CyclePhase.Liminal;
      case 'dream_propose':
        return phase === CyclePhase.CoCreation;
      default:
        return true;
    }
  }

  async isFinancialBlocked(): Promise<boolean> {
    const phase = await this.getCurrentPhase();
    return phase === CyclePhase.Kenosis || phase === CyclePhase.EmergentPersonhood;
  }

  async getTimeRemaining(): Promise<number> {
    const state = await this.getCurrentState();
    // This would need server-side calculation
    // For now, return a placeholder based on phase duration
    const phaseDurationDays = this.getPhaseDuration(state.currentPhase);
    const elapsed = state.phaseDay;
    const remainingDays = Math.max(0, phaseDurationDays - elapsed);
    return remainingDays * 24 * 60 * 60 * 1000; // Convert to milliseconds
  }

  async getTransitionHistory(): Promise<PhaseTransition[]> {
    return this.requestTransitionHistory();
  }

  onEvent(callback: (event: LivingProtocolEvent) => void): Unsubscribe {
    return this.subscriptions.subscribeAll(callback);
  }

  async getPhaseMetrics(phase: CyclePhase): Promise<PhaseMetrics> {
    return this.requestPhaseMetrics(phase);
  }

  // ---------------------------------------------------------------------------
  // Typed Subscriptions
  // ---------------------------------------------------------------------------

  /**
   * Subscribe to phase transition events.
   */
  onPhaseChange(callback: (event: Extract<LivingProtocolEvent, { type: 'PhaseTransitioned' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['PhaseTransitioned'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'PhaseTransitioned' }>)
    );
  }

  /**
   * Subscribe to new cycle events.
   */
  onCycleStart(callback: (event: Extract<LivingProtocolEvent, { type: 'CycleStarted' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['CycleStarted'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'CycleStarted' }>)
    );
  }

  /**
   * Subscribe to wound created events.
   */
  onWoundCreated(callback: (event: Extract<LivingProtocolEvent, { type: 'WoundCreated' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['WoundCreated'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'WoundCreated' }>)
    );
  }

  /**
   * Subscribe to wound phase advancement events.
   */
  onWoundAdvanced(callback: (event: Extract<LivingProtocolEvent, { type: 'WoundPhaseAdvanced' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['WoundPhaseAdvanced'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'WoundPhaseAdvanced' }>)
    );
  }

  /**
   * Subscribe to kenosis commitment events.
   */
  onKenosis(callback: (event: Extract<LivingProtocolEvent, { type: 'KenosisCommitted' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['KenosisCommitted'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'KenosisCommitted' }>)
    );
  }

  /**
   * Subscribe to entanglement events.
   */
  onEntanglement(callback: (event: Extract<LivingProtocolEvent, { type: 'EntanglementFormed' | 'EntanglementDecayed' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['EntanglementFormed', 'EntanglementDecayed'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'EntanglementFormed' | 'EntanglementDecayed' }>)
    );
  }

  /**
   * Subscribe to shadow surfacing events.
   */
  onShadowSurfaced(callback: (event: Extract<LivingProtocolEvent, { type: 'ShadowSurfaced' }>) => void): Unsubscribe {
    return this.subscriptions.subscribe(
      { eventTypes: ['ShadowSurfaced'] },
      (event) => callback(event as Extract<LivingProtocolEvent, { type: 'ShadowSurfaced' }>)
    );
  }

  /**
   * Subscribe with custom filter.
   */
  subscribeWithFilter(filter: SubscriptionFilter, callback: EventCallback): Unsubscribe {
    return this.subscriptions.subscribe(filter, callback);
  }

  // ---------------------------------------------------------------------------
  // Internal Methods
  // ---------------------------------------------------------------------------

  private handleEvent(event: LivingProtocolEvent): void {
    // Update cached state if relevant
    if (event.type === 'PhaseTransitioned') {
      if (this.cachedState) {
        this.cachedState.currentPhase = event.data.to;
        this.cachedState.phaseDay = 0;
        if (event.data.to === CyclePhase.Shadow && this.cachedState.currentPhase === CyclePhase.Kenosis) {
          this.cachedState.cycleNumber++;
        }
      }
    }

    // Dispatch to subscription manager
    this.subscriptions.dispatch(event);
  }

  private handleResponse(response: ResponseMessage): void {
    // Only handle messages with an id (responses to our requests)
    if (!response || typeof response !== 'object' || !('id' in response)) {
      return;
    }

    const pending = this.pendingRequests.get(response.id);
    if (!pending) {
      return; // Not a response to one of our requests
    }

    // Clear the timeout and remove from pending
    clearTimeout(pending.timeout);
    this.pendingRequests.delete(response.id);

    // Resolve or reject based on response
    if (response.error) {
      pending.reject(new Error(`RPC Error ${response.error.code}: ${response.error.message}`));
    } else {
      pending.resolve(response.result);
    }
  }

  /**
   * Send an RPC request and wait for response.
   */
  private async request<T>(method: string, params?: unknown): Promise<T> {
    if (!this.transport.isConnected()) {
      throw new Error('Not connected to server');
    }

    const id = `${++this.requestId}`;
    const message: RequestMessage = { id, method, params };

    return new Promise<T>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, this.requestTimeoutMs);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout,
      });

      try {
        this.transport.send(message);
      } catch (error) {
        clearTimeout(timeout);
        this.pendingRequests.delete(id);
        reject(error);
      }
    });
  }

  private async requestState(): Promise<CycleState> {
    try {
      return await this.request<CycleState>('getCycleState');
    } catch {
      // Fallback to default state if request fails (e.g., server doesn't support RPC)
      return {
        cycleNumber: 1,
        currentPhase: CyclePhase.Shadow,
        phaseStarted: new Date().toISOString(),
        cycleStarted: new Date().toISOString(),
        phaseDay: 0,
      };
    }
  }

  private async requestTransitionHistory(): Promise<PhaseTransition[]> {
    try {
      return await this.request<PhaseTransition[]>('getTransitionHistory');
    } catch {
      // Fallback to empty history if request fails
      return [];
    }
  }

  private async requestPhaseMetrics(phase: CyclePhase): Promise<PhaseMetrics> {
    try {
      return await this.request<PhaseMetrics>('getPhaseMetrics', { phase });
    } catch {
      // Fallback to empty metrics if request fails
      return {
        activeAgents: 0,
        spectralK: 0,
        meanMetabolicTrust: 0,
        activeWounds: 0,
        compostingEntities: 0,
        liminalEntities: 0,
        entangledPairs: 0,
        heldUncertainties: 0,
      };
    }
  }

  private getPhaseDuration(phase: CyclePhase): number {
    const durations: Record<CyclePhase, number> = {
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
    return durations[phase];
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Create and connect a client in one call.
 */
export async function connectToLivingProtocol(
  url: string,
  options?: Partial<LivingProtocolClientConfig>
): Promise<LivingProtocolClient> {
  return LivingProtocolClient.connect({
    url,
    ...options,
  });
}
