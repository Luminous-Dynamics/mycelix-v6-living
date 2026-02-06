import { useState, useEffect, useCallback, useRef } from 'react';
import {
  LivingProtocolClient,
  CyclePhase,
  CycleState,
  PhaseTransition,
  PhaseMetrics,
  LivingProtocolEvent,
  TransportState,
} from '@mycelix/living-protocol-sdk';

/**
 * Connection configuration for the Living Protocol client.
 */
export interface UseLivingProtocolConfig {
  /** WebSocket URL for the Living Protocol server */
  url: string;
  /** Whether to auto-connect on mount */
  autoConnect?: boolean;
  /** Reconnect delay in milliseconds */
  reconnectDelayMs?: number;
}

/**
 * State returned by the useLivingProtocol hook.
 */
export interface LivingProtocolState {
  /** Current connection state */
  connectionState: TransportState;
  /** Whether connected to the server */
  isConnected: boolean;
  /** Current cycle state */
  cycleState: CycleState | null;
  /** Recent phase transitions */
  transitionHistory: PhaseTransition[];
  /** Current phase metrics */
  metrics: PhaseMetrics | null;
  /** Recent events */
  recentEvents: LivingProtocolEvent[];
  /** Connection error if any */
  error: Error | null;
}

/**
 * Actions returned by the useLivingProtocol hook.
 */
export interface LivingProtocolActions {
  /** Connect to the server */
  connect: () => Promise<void>;
  /** Disconnect from the server */
  disconnect: () => void;
  /** Refresh cycle state */
  refreshState: () => Promise<void>;
  /** Refresh transition history */
  refreshHistory: () => Promise<void>;
  /** Refresh metrics */
  refreshMetrics: () => Promise<void>;
  /** Clear recent events */
  clearEvents: () => void;
}

const DEFAULT_CONFIG: Required<UseLivingProtocolConfig> = {
  url: 'ws://localhost:8888/ws',
  autoConnect: true,
  reconnectDelayMs: 1000,
};

const MAX_RECENT_EVENTS = 50;

/**
 * React hook for connecting to and interacting with the Living Protocol.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { state, actions } = useLivingProtocol({
 *     url: 'ws://localhost:8888/ws',
 *   });
 *
 *   if (!state.isConnected) {
 *     return <div>Connecting...</div>;
 *   }
 *
 *   return (
 *     <div>
 *       <p>Current Phase: {state.cycleState?.currentPhase}</p>
 *       <p>Cycle Number: {state.cycleState?.cycleNumber}</p>
 *     </div>
 *   );
 * }
 * ```
 */
export function useLivingProtocol(
  config: UseLivingProtocolConfig
): { state: LivingProtocolState; actions: LivingProtocolActions } {
  const fullConfig = { ...DEFAULT_CONFIG, ...config };
  const clientRef = useRef<LivingProtocolClient | null>(null);

  const [connectionState, setConnectionState] = useState<TransportState>('disconnected');
  const [cycleState, setCycleState] = useState<CycleState | null>(null);
  const [transitionHistory, setTransitionHistory] = useState<PhaseTransition[]>([]);
  const [metrics, setMetrics] = useState<PhaseMetrics | null>(null);
  const [recentEvents, setRecentEvents] = useState<LivingProtocolEvent[]>([]);
  const [error, setError] = useState<Error | null>(null);

  // Connect to the server
  const connect = useCallback(async () => {
    if (clientRef.current?.isConnected()) {
      return;
    }

    try {
      setError(null);
      const client = await LivingProtocolClient.connect({
        url: fullConfig.url,
        reconnectDelayMs: fullConfig.reconnectDelayMs,
      });

      clientRef.current = client;

      // Subscribe to connection state changes
      client.onConnectionStateChange(setConnectionState);

      // Subscribe to all events
      client.onEvent((event) => {
        setRecentEvents((prev) => {
          const next = [event, ...prev];
          return next.slice(0, MAX_RECENT_EVENTS);
        });

        // Update state based on event type
        if (event.type === 'PhaseTransitioned') {
          setCycleState((prev) =>
            prev
              ? {
                  ...prev,
                  currentPhase: event.data.to,
                  phaseDay: 0,
                }
              : null
          );
          setTransitionHistory((prev) => [event.data, ...prev].slice(0, 20));
        }
      });

      // Fetch initial state
      const state = await client.getCurrentState();
      setCycleState(state);

      const history = await client.getTransitionHistory();
      setTransitionHistory(history);

      const phaseMetrics = await client.getPhaseMetrics(state.currentPhase);
      setMetrics(phaseMetrics);

      setConnectionState('connected');
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      setConnectionState('error');
    }
  }, [fullConfig.url, fullConfig.reconnectDelayMs]);

  // Disconnect from the server
  const disconnect = useCallback(() => {
    if (clientRef.current) {
      clientRef.current.disconnect();
      clientRef.current = null;
    }
    setConnectionState('disconnected');
  }, []);

  // Refresh cycle state
  const refreshState = useCallback(async () => {
    if (!clientRef.current?.isConnected()) return;

    try {
      const state = await clientRef.current.getCurrentState();
      setCycleState(state);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  // Refresh transition history
  const refreshHistory = useCallback(async () => {
    if (!clientRef.current?.isConnected()) return;

    try {
      const history = await clientRef.current.getTransitionHistory();
      setTransitionHistory(history);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  // Refresh metrics
  const refreshMetrics = useCallback(async () => {
    if (!clientRef.current?.isConnected() || !cycleState) return;

    try {
      const phaseMetrics = await clientRef.current.getPhaseMetrics(
        cycleState.currentPhase
      );
      setMetrics(phaseMetrics);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, [cycleState]);

  // Clear recent events
  const clearEvents = useCallback(() => {
    setRecentEvents([]);
  }, []);

  // Auto-connect on mount
  useEffect(() => {
    if (fullConfig.autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, []);

  // Periodic metrics refresh
  useEffect(() => {
    if (connectionState !== 'connected') return;

    const interval = setInterval(() => {
      refreshMetrics();
    }, 10000); // Refresh every 10 seconds

    return () => clearInterval(interval);
  }, [connectionState, refreshMetrics]);

  return {
    state: {
      connectionState,
      isConnected: connectionState === 'connected',
      cycleState,
      transitionHistory,
      metrics,
      recentEvents,
      error,
    },
    actions: {
      connect,
      disconnect,
      refreshState,
      refreshHistory,
      refreshMetrics,
      clearEvents,
    },
  };
}

/**
 * Hook for subscribing to specific event types.
 */
export function useLivingProtocolEvents(
  client: LivingProtocolClient | null,
  eventTypes: LivingProtocolEvent['type'][],
  callback: (event: LivingProtocolEvent) => void
): void {
  useEffect(() => {
    if (!client) return;

    const unsubscribe = client.subscribeWithFilter(
      { eventTypes },
      callback
    );

    return unsubscribe;
  }, [client, eventTypes, callback]);
}

/**
 * Get phase display information.
 */
export function getPhaseInfo(phase: CyclePhase): {
  name: string;
  description: string;
  color: string;
  duration: number;
} {
  const phaseInfo: Record<CyclePhase, { name: string; description: string; color: string; duration: number }> = {
    [CyclePhase.Shadow]: {
      name: 'Shadow',
      description: 'Integration of suppressed content',
      color: '#4a4a6a',
      duration: 2,
    },
    [CyclePhase.Composting]: {
      name: 'Composting',
      description: 'Decomposition and nutrient extraction',
      color: '#6b4423',
      duration: 5,
    },
    [CyclePhase.Liminal]: {
      name: 'Liminal',
      description: 'Threshold state between identities',
      color: '#7c4dff',
      duration: 3,
    },
    [CyclePhase.NegativeCapability]: {
      name: 'Negative Capability',
      description: 'Holding uncertainty without resolution',
      color: '#607d8b',
      duration: 3,
    },
    [CyclePhase.Eros]: {
      name: 'Eros',
      description: 'Attraction and creative tension',
      color: '#e91e63',
      duration: 4,
    },
    [CyclePhase.CoCreation]: {
      name: 'Co-Creation',
      description: 'Collaborative emergence',
      color: '#4caf50',
      duration: 7,
    },
    [CyclePhase.Beauty]: {
      name: 'Beauty',
      description: 'Aesthetic validation',
      color: '#ff9800',
      duration: 2,
    },
    [CyclePhase.EmergentPersonhood]: {
      name: 'Emergent Personhood',
      description: 'Network consciousness assessment',
      color: '#00bcd4',
      duration: 1,
    },
    [CyclePhase.Kenosis]: {
      name: 'Kenosis',
      description: 'Voluntary release and emptying',
      color: '#9c27b0',
      duration: 1,
    },
  };

  return phaseInfo[phase];
}
