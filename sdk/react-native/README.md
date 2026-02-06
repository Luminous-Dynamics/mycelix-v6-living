# @mycelix/react-native-sdk

React Native SDK for the Living Protocol - a biologically-inspired protocol for decentralized systems.

## Installation

```bash
npm install @mycelix/react-native-sdk
# or
yarn add @mycelix/react-native-sdk
```

## Quick Start

```tsx
import React from 'react';
import { View, Text } from 'react-native';
import { useLivingProtocol, CyclePhase } from '@mycelix/react-native-sdk';

function App() {
  const {
    cycleState,
    connectionState,
    isConnected,
    error,
  } = useLivingProtocol({
    url: 'wss://your-server.com/living-protocol',
  });

  if (!isConnected) {
    return <Text>Connecting...</Text>;
  }

  if (error) {
    return <Text>Error: {error.message}</Text>;
  }

  return (
    <View>
      <Text>Phase: {cycleState?.phase}</Text>
      <Text>Cycle: {cycleState?.cycleNumber}</Text>
      <Text>Progress: {Math.round((cycleState?.progress ?? 0) * 100)}%</Text>
    </View>
  );
}
```

## Hooks

### useLivingProtocol

Main hook for managing Living Protocol connection and state.

```tsx
import { useLivingProtocol } from '@mycelix/react-native-sdk';

function MyComponent() {
  const {
    // State
    cycleState,        // Current cycle state
    connectionState,   // Connection status
    isConnected,       // Boolean connection check
    isLoading,         // Loading state
    error,             // Any errors

    // Actions
    connect,           // Connect to server
    disconnect,        // Disconnect from server
    refresh,           // Refresh cycle state
    subscribe,         // Subscribe to events

    // Client
    client,            // Raw client instance
  } = useLivingProtocol({
    url: 'wss://server.com/ws',
    autoConnect: true,           // Auto-connect on mount
    autoReconnect: true,         // Auto-reconnect on disconnect
    reconnectInterval: 3000,     // Reconnect delay (ms)
    maxReconnectAttempts: 10,    // Max reconnect attempts
    pollInterval: 0,             // Polling interval (0 = disabled)
  });
}
```

### useCycleState

Hook for accessing and reacting to cycle state with smooth progress updates.

```tsx
import { useCycleState, CyclePhase } from '@mycelix/react-native-sdk';

function CycleDisplay({ client }) {
  const {
    state,
    phase,
    cycleNumber,
    progress,        // Smooth progress (0-1)
    timeRemaining,   // Time remaining in ms
    isPhase,         // Check if specific phase is active
    isLoading,
    refresh,
  } = useCycleState({
    client,
    autoRefresh: true,
    watchPhases: [CyclePhase.GROWTH, CyclePhase.FRUITING],
  });

  return (
    <View>
      <Text>Phase: {phase}</Text>
      <ProgressBar progress={progress} />
      <Text>Time remaining: {Math.floor(timeRemaining / 1000)}s</Text>
      {isPhase(CyclePhase.FRUITING) && <Text>Fruiting in progress!</Text>}
    </View>
  );
}
```

### useTimeRemaining

Convenience hook for formatted time remaining.

```tsx
import { useTimeRemaining } from '@mycelix/react-native-sdk';

function TimeDisplay({ client }) {
  const { timeRemaining, formatted } = useTimeRemaining(client);

  return <Text>Time left: {formatted}</Text>; // "5m 30s"
}
```

### usePhaseEvents

Hook for subscribing to and managing phase events.

```tsx
import { usePhaseEvents, CyclePhase } from '@mycelix/react-native-sdk';

function EventLog({ client }) {
  const {
    events,            // All events (newest first)
    lastEvent,         // Most recent event
    phaseTransitions,  // Phase transition events only
    cycleCompletions,  // Cycle completion events only
    clearEvents,       // Clear event history
    eventCounts,       // Count by event type
  } = usePhaseEvents({
    client,
    eventTypes: ['phase_transition', 'cycle_complete'],
    phases: [CyclePhase.SPORULATION],
    maxHistory: 50,
    enabled: true,
  });

  return (
    <FlatList
      data={events}
      renderItem={({ item }) => (
        <Text>{item.type} at {item.timestamp}</Text>
      )}
    />
  );
}
```

### Event Listener Hooks

Convenience hooks for specific event types.

```tsx
import {
  useOnPhaseEnter,
  useOnPhaseExit,
  useOnCycleComplete,
  useEventListener,
  CyclePhase,
} from '@mycelix/react-native-sdk';

function Notifications({ client }) {
  // Execute when entering a phase
  useOnPhaseEnter(client, CyclePhase.FRUITING, (event) => {
    showNotification('Fruiting phase started!');
  });

  // Execute when leaving a phase
  useOnPhaseExit(client, CyclePhase.GROWTH, (event) => {
    console.log('Growth phase completed');
  });

  // Execute when cycle completes
  useOnCycleComplete(client, (event) => {
    showNotification(`Cycle ${event.cycleNumber} complete!`);
  });

  // Generic event listener
  useEventListener(client, 'error', (event) => {
    console.error('Protocol error:', event.message);
  });

  return null;
}
```

## Direct Client Usage

For more control, use the client directly.

```tsx
import { createClient, LivingProtocolClient } from '@mycelix/react-native-sdk';

const client = createClient({
  url: 'wss://server.com/ws',
  autoReconnect: true,
});

// Connect
await client.connect();

// Get state
const state = await client.getCycleState();
const phase = await client.getCurrentPhase();
const cycleNumber = await client.getCycleNumber();
const progress = await client.getPhaseProgress();
const timeRemaining = await client.getTimeRemaining();
const history = await client.getCycleHistory(10);

// Subscribe to events
const subscription = client.subscribe(
  (event) => {
    console.log('Event:', event);
  },
  {
    eventTypes: ['phase_transition', 'cycle_complete'],
    phases: [CyclePhase.FRUITING],
  }
);

// Unsubscribe
subscription.unsubscribe();

// Disconnect
client.disconnect();
```

## Types

### CyclePhase

```typescript
enum CyclePhase {
  DORMANT = 'dormant',
  GERMINATION = 'germination',
  GROWTH = 'growth',
  FRUITING = 'fruiting',
  SPORULATION = 'sporulation',
}
```

### CycleState

```typescript
interface CycleState {
  phase: CyclePhase;
  cycleNumber: number;
  phaseStartTime: number;
  phaseEndTime: number;
  phaseDuration: number;
  progress: number;
  metadata?: Record<string, unknown>;
}
```

### Events

```typescript
interface PhaseTransitionEvent {
  type: 'phase_transition';
  fromPhase: CyclePhase;
  toPhase: CyclePhase;
  cycleNumber: number;
  timestamp: number;
}

interface CycleCompletionEvent {
  type: 'cycle_complete';
  cycleNumber: number;
  duration: number;
  timestamp: number;
}

interface StateUpdateEvent {
  type: 'state_update';
  state: CycleState;
  timestamp: number;
}

interface ErrorEvent {
  type: 'error';
  code: string;
  message: string;
  timestamp: number;
}
```

### ConnectionState

```typescript
enum ConnectionState {
  DISCONNECTED = 'disconnected',
  CONNECTING = 'connecting',
  CONNECTED = 'connected',
  RECONNECTING = 'reconnecting',
  ERROR = 'error',
}
```

## Complete Example

```tsx
import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  ActivityIndicator,
} from 'react-native';
import {
  useLivingProtocol,
  useCycleState,
  useOnCycleComplete,
  CyclePhase,
} from '@mycelix/react-native-sdk';

const PHASE_COLORS = {
  [CyclePhase.DORMANT]: '#666',
  [CyclePhase.GERMINATION]: '#4CAF50',
  [CyclePhase.GROWTH]: '#8BC34A',
  [CyclePhase.FRUITING]: '#FF9800',
  [CyclePhase.SPORULATION]: '#9C27B0',
};

function LivingProtocolDashboard() {
  const { client, isConnected, error, connect, disconnect } = useLivingProtocol({
    url: 'wss://mycelix.example.com/ws',
  });

  const { phase, cycleNumber, progress, timeRemaining } = useCycleState({
    client,
  });

  useOnCycleComplete(client, (event) => {
    console.log(`Cycle ${event.cycleNumber} completed in ${event.duration}ms`);
  });

  if (error) {
    return (
      <View style={styles.container}>
        <Text style={styles.error}>Error: {error.message}</Text>
        <TouchableOpacity style={styles.button} onPress={connect}>
          <Text style={styles.buttonText}>Retry</Text>
        </TouchableOpacity>
      </View>
    );
  }

  if (!isConnected) {
    return (
      <View style={styles.container}>
        <ActivityIndicator size="large" />
        <Text style={styles.connecting}>Connecting...</Text>
      </View>
    );
  }

  const phaseColor = phase ? PHASE_COLORS[phase] : '#666';
  const minutes = Math.floor(timeRemaining / 60000);
  const seconds = Math.floor((timeRemaining % 60000) / 1000);

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Living Protocol</Text>

      <View style={[styles.phaseCard, { borderColor: phaseColor }]}>
        <Text style={[styles.phase, { color: phaseColor }]}>
          {phase?.toUpperCase()}
        </Text>
        <Text style={styles.cycle}>Cycle #{cycleNumber}</Text>
      </View>

      <View style={styles.progressContainer}>
        <View
          style={[
            styles.progressBar,
            { width: `${progress * 100}%`, backgroundColor: phaseColor },
          ]}
        />
      </View>

      <Text style={styles.time}>
        {minutes}:{seconds.toString().padStart(2, '0')} remaining
      </Text>

      <TouchableOpacity style={styles.button} onPress={disconnect}>
        <Text style={styles.buttonText}>Disconnect</Text>
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 20,
    backgroundColor: '#1a1a1a',
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#fff',
    marginBottom: 30,
  },
  phaseCard: {
    padding: 30,
    borderRadius: 15,
    borderWidth: 2,
    alignItems: 'center',
    backgroundColor: '#2a2a2a',
    marginBottom: 20,
  },
  phase: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  cycle: {
    fontSize: 16,
    color: '#aaa',
    marginTop: 5,
  },
  progressContainer: {
    width: '100%',
    height: 8,
    backgroundColor: '#333',
    borderRadius: 4,
    marginBottom: 10,
    overflow: 'hidden',
  },
  progressBar: {
    height: '100%',
    borderRadius: 4,
  },
  time: {
    fontSize: 18,
    color: '#fff',
    marginBottom: 30,
  },
  button: {
    backgroundColor: '#444',
    paddingHorizontal: 30,
    paddingVertical: 12,
    borderRadius: 8,
  },
  buttonText: {
    color: '#fff',
    fontSize: 16,
  },
  connecting: {
    color: '#aaa',
    marginTop: 20,
  },
  error: {
    color: '#f44336',
    marginBottom: 20,
  },
});

export default LivingProtocolDashboard;
```

## License

MIT
