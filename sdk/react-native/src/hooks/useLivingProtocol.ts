/**
 * useLivingProtocol - Main React hook for Living Protocol state management
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { LivingProtocolClient, createClient } from '../client';
import {
  ClientOptions,
  ConnectionState,
  CycleState,
  LivingProtocolEvent,
  SubscriptionOptions,
} from '../types';

export interface UseLivingProtocolOptions extends ClientOptions {
  /**
   * Whether to automatically connect on mount
   * @default true
   */
  autoConnect?: boolean;

  /**
   * Poll interval for state updates (ms)
   * Set to 0 to disable polling (use subscriptions only)
   * @default 0
   */
  pollInterval?: number;
}

export interface UseLivingProtocolResult {
  /**
   * The current cycle state
   */
  cycleState: CycleState | null;

  /**
   * The current connection state
   */
  connectionState: ConnectionState;

  /**
   * Whether the client is connected
   */
  isConnected: boolean;

  /**
   * Whether the state is loading
   */
  isLoading: boolean;

  /**
   * Any error that occurred
   */
  error: Error | null;

  /**
   * Connect to the server
   */
  connect: () => Promise<void>;

  /**
   * Disconnect from the server
   */
  disconnect: () => void;

  /**
   * Refresh the cycle state
   */
  refresh: () => Promise<void>;

  /**
   * Subscribe to events
   */
  subscribe: (
    callback: (event: LivingProtocolEvent) => void,
    options?: SubscriptionOptions
  ) => () => void;

  /**
   * The underlying client instance
   */
  client: LivingProtocolClient;
}

/**
 * Main hook for managing Living Protocol connection and state
 */
export function useLivingProtocol(
  options: UseLivingProtocolOptions
): UseLivingProtocolResult {
  const { autoConnect = true, pollInterval = 0, ...clientOptions } = options;

  // Create client once
  const clientRef = useRef<LivingProtocolClient | null>(null);
  if (!clientRef.current) {
    clientRef.current = createClient(clientOptions);
  }
  const client = clientRef.current;

  // State
  const [cycleState, setCycleState] = useState<CycleState | null>(null);
  const [connectionState, setConnectionState] = useState<ConnectionState>(
    ConnectionState.DISCONNECTED
  );
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Derived state
  const isConnected = connectionState === ConnectionState.CONNECTED;

  // Fetch cycle state
  const fetchCycleState = useCallback(async () => {
    if (!isConnected) return;

    try {
      const state = await client.getCycleState();
      setCycleState(state);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch state'));
    }
  }, [client, isConnected]);

  // Connect handler
  const connect = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      await client.connect();
      await fetchCycleState();
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Connection failed'));
    } finally {
      setIsLoading(false);
    }
  }, [client, fetchCycleState]);

  // Disconnect handler
  const disconnect = useCallback(() => {
    client.disconnect();
    setCycleState(null);
    setError(null);
  }, [client]);

  // Refresh handler
  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      await fetchCycleState();
    } finally {
      setIsLoading(false);
    }
  }, [fetchCycleState]);

  // Subscribe wrapper
  const subscribe = useCallback(
    (
      callback: (event: LivingProtocolEvent) => void,
      subscriptionOptions?: SubscriptionOptions
    ) => {
      const subscription = client.subscribe(callback, subscriptionOptions);
      return subscription.unsubscribe;
    },
    [client]
  );

  // Connection state listener
  useEffect(() => {
    const unsubscribe = client.onConnectionStateChange((state) => {
      setConnectionState(state);
    });

    return unsubscribe;
  }, [client]);

  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      // Don't disconnect on unmount by default to allow reconnection
      // The client will be garbage collected when the component using it unmounts
    };
  }, [autoConnect, connect]);

  // Subscribe to state updates
  useEffect(() => {
    if (!isConnected) return;

    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'state_update') {
          setCycleState(event.state);
        } else if (event.type === 'phase_transition') {
          // Refresh state on phase transition
          fetchCycleState();
        }
      },
      { eventTypes: ['state_update', 'phase_transition'] }
    );

    return subscription.unsubscribe;
  }, [client, isConnected, fetchCycleState]);

  // Polling (if enabled)
  useEffect(() => {
    if (!isConnected || pollInterval <= 0) return;

    const timer = setInterval(fetchCycleState, pollInterval);
    return () => clearInterval(timer);
  }, [isConnected, pollInterval, fetchCycleState]);

  // Memoize result to prevent unnecessary re-renders
  return useMemo(
    () => ({
      cycleState,
      connectionState,
      isConnected,
      isLoading,
      error,
      connect,
      disconnect,
      refresh,
      subscribe,
      client,
    }),
    [
      cycleState,
      connectionState,
      isConnected,
      isLoading,
      error,
      connect,
      disconnect,
      refresh,
      subscribe,
      client,
    ]
  );
}
