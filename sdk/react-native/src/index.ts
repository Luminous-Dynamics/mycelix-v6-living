/**
 * @mycelix/react-native-sdk
 * React Native SDK for the Living Protocol
 */

// Client
export { LivingProtocolClient, createClient } from './client';

// Hooks
export {
  useLivingProtocol,
  type UseLivingProtocolOptions,
  type UseLivingProtocolResult,
} from './hooks/useLivingProtocol';

export {
  useCycleState,
  useIsPhase,
  useTimeRemaining,
  type UseCycleStateOptions,
  type UseCycleStateResult,
} from './hooks/useCycleState';

export {
  usePhaseEvents,
  useEventListener,
  usePhaseTransitionListener,
  useOnPhaseEnter,
  useOnPhaseExit,
  useOnCycleComplete,
  type UsePhaseEventsOptions,
  type UsePhaseEventsResult,
} from './hooks/usePhaseEvents';

// Types
export {
  CyclePhase,
  ConnectionState,
  type CycleState,
  type PhaseTransitionEvent,
  type CycleCompletionEvent,
  type StateUpdateEvent,
  type ErrorEvent,
  type LivingProtocolEvent,
  type EventType,
  type SubscriptionOptions,
  type Subscription,
  type ClientOptions,
} from './types';
