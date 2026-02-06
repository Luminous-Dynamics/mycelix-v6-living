/**
 * Living Protocol Client for React Native
 * Handles WebSocket connection and RPC communication
 */

import {
  ClientOptions,
  ConnectionState,
  CyclePhase,
  CycleState,
  EventType,
  LivingProtocolEvent,
  RPCNotification,
  RPCRequest,
  RPCResponse,
  Subscription,
  SubscriptionOptions,
} from './types';

type EventCallback = (event: LivingProtocolEvent) => void;
type ConnectionCallback = (state: ConnectionState) => void;

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

interface EventSubscription {
  id: string;
  callback: EventCallback;
  options: SubscriptionOptions;
}

/**
 * LivingProtocolClient - Main client for connecting to the Living Protocol
 */
export class LivingProtocolClient {
  private options: Required<ClientOptions>;
  private ws: WebSocket | null = null;
  private connectionState: ConnectionState = ConnectionState.DISCONNECTED;
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private requestId = 0;
  private pendingRequests: Map<string | number, PendingRequest> = new Map();
  private subscriptions: Map<string, EventSubscription> = new Map();
  private connectionListeners: Set<ConnectionCallback> = new Set();
  private serverSubscriptionId: string | null = null;

  constructor(options: ClientOptions) {
    this.options = {
      url: options.url,
      autoReconnect: options.autoReconnect ?? true,
      reconnectInterval: options.reconnectInterval ?? 3000,
      maxReconnectAttempts: options.maxReconnectAttempts ?? 10,
      heartbeatInterval: options.heartbeatInterval ?? 30000,
      connectionTimeout: options.connectionTimeout ?? 10000,
    };
  }

  /**
   * Connect to the Living Protocol server
   */
  async connect(): Promise<void> {
    if (this.connectionState === ConnectionState.CONNECTED) {
      return;
    }

    return new Promise((resolve, reject) => {
      this.setConnectionState(ConnectionState.CONNECTING);

      const timeout = setTimeout(() => {
        this.ws?.close();
        reject(new Error('Connection timeout'));
      }, this.options.connectionTimeout);

      try {
        this.ws = new WebSocket(this.options.url);

        this.ws.onopen = () => {
          clearTimeout(timeout);
          this.reconnectAttempts = 0;
          this.setConnectionState(ConnectionState.CONNECTED);
          this.startHeartbeat();
          this.resubscribeAll();
          resolve();
        };

        this.ws.onclose = (event) => {
          clearTimeout(timeout);
          this.handleDisconnect(event);
        };

        this.ws.onerror = (error) => {
          clearTimeout(timeout);
          this.setConnectionState(ConnectionState.ERROR);
          reject(new Error('WebSocket error'));
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data);
        };
      } catch (error) {
        clearTimeout(timeout);
        this.setConnectionState(ConnectionState.ERROR);
        reject(error);
      }
    });
  }

  /**
   * Disconnect from the server
   */
  disconnect(): void {
    this.options.autoReconnect = false;
    this.stopHeartbeat();
    this.clearReconnectTimer();

    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }

    this.setConnectionState(ConnectionState.DISCONNECTED);
    this.pendingRequests.forEach((req) => {
      clearTimeout(req.timeout);
      req.reject(new Error('Client disconnected'));
    });
    this.pendingRequests.clear();
  }

  /**
   * Get the current connection state
   */
  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  /**
   * Subscribe to connection state changes
   */
  onConnectionStateChange(callback: ConnectionCallback): () => void {
    this.connectionListeners.add(callback);
    return () => this.connectionListeners.delete(callback);
  }

  /**
   * Get the current cycle state
   */
  async getCycleState(): Promise<CycleState> {
    return this.rpcCall<CycleState>('getCycleState');
  }

  /**
   * Get the current phase
   */
  async getCurrentPhase(): Promise<CyclePhase> {
    return this.rpcCall<CyclePhase>('getCurrentPhase');
  }

  /**
   * Get the current cycle number
   */
  async getCycleNumber(): Promise<number> {
    return this.rpcCall<number>('getCycleNumber');
  }

  /**
   * Get the phase progress (0-1)
   */
  async getPhaseProgress(): Promise<number> {
    return this.rpcCall<number>('getPhaseProgress');
  }

  /**
   * Get time remaining in current phase (ms)
   */
  async getTimeRemaining(): Promise<number> {
    return this.rpcCall<number>('getTimeRemaining');
  }

  /**
   * Get the cycle history
   */
  async getCycleHistory(limit?: number): Promise<CycleState[]> {
    return this.rpcCall<CycleState[]>('getCycleHistory', { limit });
  }

  /**
   * Advance to the next phase (if allowed)
   */
  async advancePhase(): Promise<CycleState> {
    return this.rpcCall<CycleState>('advancePhase');
  }

  /**
   * Subscribe to Living Protocol events
   */
  subscribe(
    callback: EventCallback,
    options: SubscriptionOptions = {}
  ): Subscription {
    const id = this.generateSubscriptionId();

    const subscription: EventSubscription = {
      id,
      callback,
      options,
    };

    this.subscriptions.set(id, subscription);

    // If connected, set up server-side subscription
    if (this.connectionState === ConnectionState.CONNECTED) {
      this.setupServerSubscription();
    }

    return {
      id,
      unsubscribe: () => {
        this.subscriptions.delete(id);
        if (this.subscriptions.size === 0) {
          this.teardownServerSubscription();
        }
      },
    };
  }

  /**
   * Subscribe to specific event types
   */
  subscribeToEvents(
    eventTypes: EventType[],
    callback: EventCallback
  ): Subscription {
    return this.subscribe(callback, { eventTypes });
  }

  /**
   * Subscribe to specific phases
   */
  subscribeToPhases(
    phases: CyclePhase[],
    callback: EventCallback
  ): Subscription {
    return this.subscribe(callback, { phases });
  }

  private setConnectionState(state: ConnectionState): void {
    this.connectionState = state;
    this.connectionListeners.forEach((cb) => cb(state));
  }

  private handleDisconnect(event: CloseEvent): void {
    this.stopHeartbeat();
    this.ws = null;
    this.serverSubscriptionId = null;

    if (this.options.autoReconnect && this.reconnectAttempts < this.options.maxReconnectAttempts) {
      this.setConnectionState(ConnectionState.RECONNECTING);
      this.scheduleReconnect();
    } else {
      this.setConnectionState(ConnectionState.DISCONNECTED);
    }
  }

  private scheduleReconnect(): void {
    this.clearReconnectTimer();

    const delay = Math.min(
      this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts),
      30000
    );

    this.reconnectTimer = setTimeout(async () => {
      this.reconnectAttempts++;
      try {
        await this.connect();
      } catch (error) {
        // Connection will handle retry
      }
    }, delay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      if (this.connectionState === ConnectionState.CONNECTED) {
        this.rpcCall('ping').catch(() => {
          // Heartbeat failure will trigger reconnect via onclose
        });
      }
    }, this.options.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private handleMessage(data: string): void {
    try {
      const message = JSON.parse(data);

      // Check if it's a notification (event)
      if (!message.id && message.method) {
        this.handleNotification(message as RPCNotification);
        return;
      }

      // Handle RPC response
      const response = message as RPCResponse;
      const pending = this.pendingRequests.get(response.id);

      if (pending) {
        clearTimeout(pending.timeout);
        this.pendingRequests.delete(response.id);

        if (response.error) {
          pending.reject(new Error(response.error.message));
        } else {
          pending.resolve(response.result);
        }
      }
    } catch (error) {
      console.error('Failed to parse message:', error);
    }
  }

  private handleNotification(notification: RPCNotification): void {
    const event = notification.params;

    this.subscriptions.forEach((subscription) => {
      if (this.eventMatchesOptions(event, subscription.options)) {
        try {
          subscription.callback(event);
        } catch (error) {
          console.error('Subscription callback error:', error);
        }
      }
    });
  }

  private eventMatchesOptions(
    event: LivingProtocolEvent,
    options: SubscriptionOptions
  ): boolean {
    // Check event type filter
    if (options.eventTypes && options.eventTypes.length > 0) {
      if (!options.eventTypes.includes(event.type)) {
        return false;
      }
    }

    // Check phase filter (for phase-related events)
    if (options.phases && options.phases.length > 0) {
      if (event.type === 'phase_transition') {
        if (!options.phases.includes(event.toPhase)) {
          return false;
        }
      } else if (event.type === 'state_update') {
        if (!options.phases.includes(event.state.phase)) {
          return false;
        }
      }
    }

    // Check cycle number filter
    if (options.cycleNumbers && options.cycleNumbers.length > 0) {
      if ('cycleNumber' in event) {
        if (!options.cycleNumbers.includes(event.cycleNumber)) {
          return false;
        }
      } else if (event.type === 'state_update') {
        if (!options.cycleNumbers.includes(event.state.cycleNumber)) {
          return false;
        }
      }
    }

    return true;
  }

  private async setupServerSubscription(): Promise<void> {
    if (this.serverSubscriptionId) {
      return;
    }

    try {
      const result = await this.rpcCall<{ subscriptionId: string }>('subscribe', {
        events: ['phase_transition', 'cycle_complete', 'state_update', 'error'],
      });
      this.serverSubscriptionId = result.subscriptionId;
    } catch (error) {
      console.error('Failed to setup server subscription:', error);
    }
  }

  private async teardownServerSubscription(): Promise<void> {
    if (!this.serverSubscriptionId) {
      return;
    }

    try {
      await this.rpcCall('unsubscribe', {
        subscriptionId: this.serverSubscriptionId,
      });
    } catch (error) {
      console.error('Failed to teardown server subscription:', error);
    } finally {
      this.serverSubscriptionId = null;
    }
  }

  private async resubscribeAll(): Promise<void> {
    if (this.subscriptions.size > 0) {
      this.serverSubscriptionId = null;
      await this.setupServerSubscription();
    }
  }

  private async rpcCall<T>(
    method: string,
    params?: Record<string, unknown>
  ): Promise<T> {
    if (this.connectionState !== ConnectionState.CONNECTED || !this.ws) {
      throw new Error('Not connected');
    }

    const id = ++this.requestId;

    const request: RPCRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, 30000);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout,
      });

      this.ws!.send(JSON.stringify(request));
    });
  }

  private generateSubscriptionId(): string {
    return `sub_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
}

/**
 * Create a new Living Protocol client
 */
export function createClient(options: ClientOptions): LivingProtocolClient {
  return new LivingProtocolClient(options);
}
