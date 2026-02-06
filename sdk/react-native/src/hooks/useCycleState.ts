/**
 * useCycleState - Hook for accessing and reacting to cycle state
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import { LivingProtocolClient } from '../client';
import { CyclePhase, CycleState } from '../types';

export interface UseCycleStateOptions {
  /**
   * The client instance to use
   */
  client: LivingProtocolClient;

  /**
   * Whether to auto-refresh on phase transitions
   * @default true
   */
  autoRefresh?: boolean;

  /**
   * Phases to specifically watch for
   */
  watchPhases?: CyclePhase[];
}

export interface UseCycleStateResult {
  /**
   * The current cycle state
   */
  state: CycleState | null;

  /**
   * Current phase
   */
  phase: CyclePhase | null;

  /**
   * Current cycle number
   */
  cycleNumber: number | null;

  /**
   * Progress through current phase (0-1)
   */
  progress: number;

  /**
   * Time remaining in current phase (ms)
   */
  timeRemaining: number;

  /**
   * Whether the specified phase is active
   */
  isPhase: (phase: CyclePhase) => boolean;

  /**
   * Whether currently loading
   */
  isLoading: boolean;

  /**
   * Refresh the state
   */
  refresh: () => Promise<void>;
}

/**
 * Hook for managing cycle state
 */
export function useCycleState(
  options: UseCycleStateOptions
): UseCycleStateResult {
  const { client, autoRefresh = true, watchPhases } = options;

  const [state, setState] = useState<CycleState | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [localProgress, setLocalProgress] = useState(0);
  const [localTimeRemaining, setLocalTimeRemaining] = useState(0);

  // Fetch state from server
  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const cycleState = await client.getCycleState();
      setState(cycleState);
      setLocalProgress(cycleState.progress);
      setLocalTimeRemaining(
        Math.max(0, cycleState.phaseEndTime - Date.now())
      );
    } catch (error) {
      console.error('Failed to fetch cycle state:', error);
    } finally {
      setIsLoading(false);
    }
  }, [client]);

  // Initial fetch when client connects
  useEffect(() => {
    const unsubscribe = client.onConnectionStateChange((connectionState) => {
      if (connectionState === 'connected') {
        refresh();
      }
    });

    // If already connected, fetch immediately
    if (client.getConnectionState() === 'connected') {
      refresh();
    }

    return unsubscribe;
  }, [client, refresh]);

  // Subscribe to phase transitions
  useEffect(() => {
    if (!autoRefresh) return;

    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'phase_transition') {
          // Check if we should respond to this phase
          if (!watchPhases || watchPhases.includes(event.toPhase)) {
            refresh();
          }
        } else if (event.type === 'state_update') {
          setState(event.state);
          setLocalProgress(event.state.progress);
          setLocalTimeRemaining(
            Math.max(0, event.state.phaseEndTime - Date.now())
          );
        }
      },
      {
        eventTypes: ['phase_transition', 'state_update'],
        phases: watchPhases,
      }
    );

    return subscription.unsubscribe;
  }, [client, autoRefresh, watchPhases, refresh]);

  // Local progress/time updates (smooth interpolation)
  useEffect(() => {
    if (!state) return;

    const updateInterval = setInterval(() => {
      const now = Date.now();
      const elapsed = now - state.phaseStartTime;
      const duration = state.phaseDuration;

      const newProgress = Math.min(1, Math.max(0, elapsed / duration));
      const newTimeRemaining = Math.max(0, state.phaseEndTime - now);

      setLocalProgress(newProgress);
      setLocalTimeRemaining(newTimeRemaining);
    }, 100); // Update every 100ms for smooth progress

    return () => clearInterval(updateInterval);
  }, [state]);

  // Phase check helper
  const isPhase = useCallback(
    (phase: CyclePhase): boolean => {
      return state?.phase === phase;
    },
    [state]
  );

  return useMemo(
    () => ({
      state,
      phase: state?.phase ?? null,
      cycleNumber: state?.cycleNumber ?? null,
      progress: localProgress,
      timeRemaining: localTimeRemaining,
      isPhase,
      isLoading,
      refresh,
    }),
    [state, localProgress, localTimeRemaining, isPhase, isLoading, refresh]
  );
}

/**
 * Convenience hook for checking if a specific phase is active
 */
export function useIsPhase(
  client: LivingProtocolClient,
  phase: CyclePhase
): boolean {
  const { isPhase } = useCycleState({ client, watchPhases: [phase] });
  return isPhase(phase);
}

/**
 * Hook for getting formatted time remaining
 */
export function useTimeRemaining(client: LivingProtocolClient): {
  timeRemaining: number;
  formatted: string;
} {
  const { timeRemaining } = useCycleState({ client });

  const formatted = useMemo(() => {
    const seconds = Math.floor(timeRemaining / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
      return `${hours}h ${minutes % 60}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds % 60}s`;
    } else {
      return `${seconds}s`;
    }
  }, [timeRemaining]);

  return { timeRemaining, formatted };
}
