/**
 * Living Protocol Types for React Native SDK
 */

/**
 * Cycle phases in the Living Protocol
 */
export enum CyclePhase {
  DORMANT = 'dormant',
  GERMINATION = 'germination',
  GROWTH = 'growth',
  FRUITING = 'fruiting',
  SPORULATION = 'sporulation',
}

/**
 * Current state of the cycle
 */
export interface CycleState {
  phase: CyclePhase;
  cycleNumber: number;
  phaseStartTime: number;
  phaseEndTime: number;
  phaseDuration: number;
  progress: number;
  metadata?: Record<string, unknown>;
}

/**
 * Phase transition event
 */
export interface PhaseTransitionEvent {
  type: 'phase_transition';
  fromPhase: CyclePhase;
  toPhase: CyclePhase;
  cycleNumber: number;
  timestamp: number;
}

/**
 * Cycle completion event
 */
export interface CycleCompletionEvent {
  type: 'cycle_complete';
  cycleNumber: number;
  duration: number;
  timestamp: number;
}

/**
 * State update event
 */
export interface StateUpdateEvent {
  type: 'state_update';
  state: CycleState;
  timestamp: number;
}

/**
 * Error event
 */
export interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  timestamp: number;
}

/**
 * Union type for all events
 */
export type LivingProtocolEvent =
  | PhaseTransitionEvent
  | CycleCompletionEvent
  | StateUpdateEvent
  | ErrorEvent;

/**
 * Event types for filtering
 */
export type EventType = LivingProtocolEvent['type'];

/**
 * Subscription options
 */
export interface SubscriptionOptions {
  eventTypes?: EventType[];
  phases?: CyclePhase[];
  cycleNumbers?: number[];
}

/**
 * Subscription handle
 */
export interface Subscription {
  id: string;
  unsubscribe: () => void;
}

/**
 * Client configuration options
 */
export interface ClientOptions {
  url: string;
  autoReconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
  connectionTimeout?: number;
}

/**
 * Connection state
 */
export enum ConnectionState {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  RECONNECTING = 'reconnecting',
  ERROR = 'error',
}

/**
 * RPC request structure
 */
export interface RPCRequest {
  jsonrpc: '2.0';
  id: string | number;
  method: string;
  params?: Record<string, unknown>;
}

/**
 * RPC response structure
 */
export interface RPCResponse<T = unknown> {
  jsonrpc: '2.0';
  id: string | number;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

/**
 * RPC notification structure (for events)
 */
export interface RPCNotification {
  jsonrpc: '2.0';
  method: string;
  params: LivingProtocolEvent;
}
