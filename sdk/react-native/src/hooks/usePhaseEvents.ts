/**
 * usePhaseEvents - Hook for subscribing to phase-specific events
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { LivingProtocolClient } from '../client';
import {
  CycleCompletionEvent,
  CyclePhase,
  EventType,
  LivingProtocolEvent,
  PhaseTransitionEvent,
  StateUpdateEvent,
} from '../types';

export interface UsePhaseEventsOptions {
  /**
   * The client instance to use
   */
  client: LivingProtocolClient;

  /**
   * Event types to subscribe to
   */
  eventTypes?: EventType[];

  /**
   * Phases to filter for
   */
  phases?: CyclePhase[];

  /**
   * Maximum number of events to keep in history
   * @default 50
   */
  maxHistory?: number;

  /**
   * Whether the subscription is enabled
   * @default true
   */
  enabled?: boolean;
}

export interface UsePhaseEventsResult {
  /**
   * All events received (newest first)
   */
  events: LivingProtocolEvent[];

  /**
   * The most recent event
   */
  lastEvent: LivingProtocolEvent | null;

  /**
   * Phase transition events only
   */
  phaseTransitions: PhaseTransitionEvent[];

  /**
   * Cycle completion events only
   */
  cycleCompletions: CycleCompletionEvent[];

  /**
   * Clear the event history
   */
  clearEvents: () => void;

  /**
   * Count of events by type
   */
  eventCounts: Record<EventType, number>;
}

/**
 * Hook for subscribing to and managing phase events
 */
export function usePhaseEvents(
  options: UsePhaseEventsOptions
): UsePhaseEventsResult {
  const {
    client,
    eventTypes,
    phases,
    maxHistory = 50,
    enabled = true,
  } = options;

  const [events, setEvents] = useState<LivingProtocolEvent[]>([]);
  const eventsRef = useRef<LivingProtocolEvent[]>([]);

  // Add event to history
  const addEvent = useCallback(
    (event: LivingProtocolEvent) => {
      eventsRef.current = [event, ...eventsRef.current].slice(0, maxHistory);
      setEvents(eventsRef.current);
    },
    [maxHistory]
  );

  // Clear events
  const clearEvents = useCallback(() => {
    eventsRef.current = [];
    setEvents([]);
  }, []);

  // Subscribe to events
  useEffect(() => {
    if (!enabled) return;

    const subscription = client.subscribe(
      (event) => {
        addEvent(event);
      },
      {
        eventTypes,
        phases,
      }
    );

    return subscription.unsubscribe;
  }, [client, enabled, eventTypes, phases, addEvent]);

  // Derived data
  const lastEvent = events[0] ?? null;

  const phaseTransitions = events.filter(
    (e): e is PhaseTransitionEvent => e.type === 'phase_transition'
  );

  const cycleCompletions = events.filter(
    (e): e is CycleCompletionEvent => e.type === 'cycle_complete'
  );

  const eventCounts = events.reduce(
    (acc, event) => {
      acc[event.type] = (acc[event.type] || 0) + 1;
      return acc;
    },
    {} as Record<EventType, number>
  );

  return {
    events,
    lastEvent,
    phaseTransitions,
    cycleCompletions,
    clearEvents,
    eventCounts,
  };
}

/**
 * Hook for listening to a specific event type
 */
export function useEventListener<T extends LivingProtocolEvent>(
  client: LivingProtocolClient,
  eventType: T['type'],
  callback: (event: T) => void,
  deps: React.DependencyList = []
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const subscription = client.subscribe(
      (event) => {
        if (event.type === eventType) {
          callbackRef.current(event as T);
        }
      },
      { eventTypes: [eventType] }
    );

    return subscription.unsubscribe;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, eventType, ...deps]);
}

/**
 * Hook for listening to phase transitions
 */
export function usePhaseTransitionListener(
  client: LivingProtocolClient,
  callback: (event: PhaseTransitionEvent) => void,
  phases?: CyclePhase[]
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'phase_transition') {
          callbackRef.current(event);
        }
      },
      {
        eventTypes: ['phase_transition'],
        phases,
      }
    );

    return subscription.unsubscribe;
  }, [client, phases]);
}

/**
 * Hook for executing code when entering a specific phase
 */
export function useOnPhaseEnter(
  client: LivingProtocolClient,
  phase: CyclePhase,
  callback: (event: PhaseTransitionEvent) => void
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'phase_transition' && event.toPhase === phase) {
          callbackRef.current(event);
        }
      },
      {
        eventTypes: ['phase_transition'],
        phases: [phase],
      }
    );

    return subscription.unsubscribe;
  }, [client, phase]);
}

/**
 * Hook for executing code when leaving a specific phase
 */
export function useOnPhaseExit(
  client: LivingProtocolClient,
  phase: CyclePhase,
  callback: (event: PhaseTransitionEvent) => void
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'phase_transition' && event.fromPhase === phase) {
          callbackRef.current(event);
        }
      },
      { eventTypes: ['phase_transition'] }
    );

    return subscription.unsubscribe;
  }, [client, phase]);
}

/**
 * Hook for executing code when a cycle completes
 */
export function useOnCycleComplete(
  client: LivingProtocolClient,
  callback: (event: CycleCompletionEvent) => void
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const subscription = client.subscribe(
      (event) => {
        if (event.type === 'cycle_complete') {
          callbackRef.current(event);
        }
      },
      { eventTypes: ['cycle_complete'] }
    );

    return subscription.unsubscribe;
  }, [client]);
}
