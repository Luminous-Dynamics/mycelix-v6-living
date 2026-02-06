/**
 * WebSocket Transport Layer
 * Manages WebSocket connections with auto-reconnect and heartbeat support.
 */

import { LivingProtocolEvent } from './cycle';

// =============================================================================
// Configuration Types
// =============================================================================

export interface TransportConfig {
  /** WebSocket server URL */
  url: string;

  /** Delay before reconnect attempt (ms) */
  reconnectDelayMs?: number;

  /** Maximum reconnect attempts before giving up */
  maxReconnectAttempts?: number;

  /** Heartbeat interval (ms) */
  heartbeatIntervalMs?: number;

  /** Connection timeout (ms) */
  connectionTimeoutMs?: number;
}

export const DEFAULT_TRANSPORT_CONFIG: Required<Omit<TransportConfig, 'url'>> = {
  reconnectDelayMs: 1000,
  maxReconnectAttempts: 10,
  heartbeatIntervalMs: 30000,
  connectionTimeoutMs: 10000,
};

// =============================================================================
// Event Types
// =============================================================================

export type TransportState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected' | 'error';

export type TransportEvent =
  | { type: 'stateChange'; state: TransportState }
  | { type: 'message'; data: unknown }
  | { type: 'error'; error: Error }
  | { type: 'reconnecting'; attempt: number; maxAttempts: number };

export type TransportEventCallback = (event: TransportEvent) => void;

// =============================================================================
// WebSocket Transport
// =============================================================================

export class WebSocketTransport {
  private ws: WebSocket | null = null;
  private config: Required<TransportConfig>;
  private state: TransportState = 'disconnected';
  private reconnectAttempts = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private listeners: Map<string, Set<TransportEventCallback>> = new Map();
  private messageListeners: Set<(event: LivingProtocolEvent) => void> = new Set();

  constructor(config: TransportConfig) {
    this.config = {
      ...DEFAULT_TRANSPORT_CONFIG,
      ...config,
    };
  }

  // ---------------------------------------------------------------------------
  // Connection Management
  // ---------------------------------------------------------------------------

  /**
   * Connect to the WebSocket server.
   */
  async connect(): Promise<void> {
    if (this.state === 'connected' || this.state === 'connecting') {
      return;
    }

    return new Promise((resolve, reject) => {
      this.setState('connecting');

      const timeout = setTimeout(() => {
        this.ws?.close();
        reject(new Error('Connection timeout'));
      }, this.config.connectionTimeoutMs);

      try {
        this.ws = new WebSocket(this.config.url);

        this.ws.onopen = () => {
          clearTimeout(timeout);
          this.reconnectAttempts = 0;
          this.setState('connected');
          this.startHeartbeat();
          resolve();
        };

        this.ws.onclose = (event) => {
          clearTimeout(timeout);
          this.stopHeartbeat();

          if (this.state !== 'disconnected') {
            this.handleReconnect();
          }
        };

        this.ws.onerror = (event) => {
          clearTimeout(timeout);
          const error = new Error('WebSocket error');
          this.emit({ type: 'error', error });
          reject(error);
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event);
        };
      } catch (error) {
        clearTimeout(timeout);
        this.setState('error');
        reject(error);
      }
    });
  }

  /**
   * Disconnect from the WebSocket server.
   */
  disconnect(): void {
    this.setState('disconnected');
    this.stopHeartbeat();
    this.stopReconnect();

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  /**
   * Check if connected.
   */
  isConnected(): boolean {
    return this.state === 'connected';
  }

  /**
   * Get current connection state.
   */
  getState(): TransportState {
    return this.state;
  }

  // ---------------------------------------------------------------------------
  // Event Subscription
  // ---------------------------------------------------------------------------

  /**
   * Subscribe to transport events.
   */
  on(eventType: string, callback: TransportEventCallback): () => void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    this.listeners.get(eventType)!.add(callback);

    // Return unsubscribe function
    return () => {
      this.listeners.get(eventType)?.delete(callback);
    };
  }

  /**
   * Subscribe to protocol events.
   */
  onProtocolEvent(callback: (event: LivingProtocolEvent) => void): () => void {
    this.messageListeners.add(callback);
    return () => {
      this.messageListeners.delete(callback);
    };
  }

  /**
   * Send a message through the WebSocket.
   */
  send(data: unknown): void {
    if (this.state !== 'connected' || !this.ws) {
      throw new Error('Not connected');
    }

    this.ws.send(JSON.stringify(data));
  }

  // ---------------------------------------------------------------------------
  // Internal Methods
  // ---------------------------------------------------------------------------

  private setState(state: TransportState): void {
    if (this.state !== state) {
      this.state = state;
      this.emit({ type: 'stateChange', state });
    }
  }

  private emit(event: TransportEvent): void {
    // Emit to specific event type listeners
    const listeners = this.listeners.get(event.type);
    if (listeners) {
      listeners.forEach(callback => callback(event));
    }

    // Emit to wildcard listeners
    const wildcardListeners = this.listeners.get('*');
    if (wildcardListeners) {
      wildcardListeners.forEach(callback => callback(event));
    }
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const data = JSON.parse(event.data);

      this.emit({ type: 'message', data });

      // If it looks like a protocol event, dispatch to protocol listeners
      if (data && typeof data === 'object' && 'type' in data) {
        this.messageListeners.forEach(callback => {
          callback(data as LivingProtocolEvent);
        });
      }
    } catch (error) {
      this.emit({ type: 'error', error: new Error(`Failed to parse message: ${error}`) });
    }
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.setState('error');
      this.emit({
        type: 'error',
        error: new Error(`Max reconnect attempts (${this.config.maxReconnectAttempts}) reached`),
      });
      return;
    }

    this.reconnectAttempts++;
    this.setState('reconnecting');
    this.emit({
      type: 'reconnecting',
      attempt: this.reconnectAttempts,
      maxAttempts: this.config.maxReconnectAttempts,
    });

    // Exponential backoff with jitter
    const delay = this.config.reconnectDelayMs * Math.pow(2, this.reconnectAttempts - 1);
    const jitter = delay * 0.1 * Math.random();

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
        // Connect will trigger another reconnect if needed
      });
    }, delay + jitter);

    // Allow Node.js to exit even if reconnect is pending
    if (this.reconnectTimer.unref) {
      this.reconnectTimer.unref();
    }
  }

  private stopReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.reconnectAttempts = 0;
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();

    this.heartbeatTimer = setInterval(() => {
      if (this.state === 'connected' && this.ws) {
        try {
          this.ws.send(JSON.stringify({ type: 'ping' }));
        } catch (error) {
          // Connection may have been lost
        }
      }
    }, this.config.heartbeatIntervalMs);

    // Allow Node.js to exit even if heartbeat is active
    if (this.heartbeatTimer.unref) {
      this.heartbeatTimer.unref();
    }
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }
}
