# Mycelix GraphQL API

The Mycelix WebSocket server includes an optional GraphQL API for querying cycle state and subscribing to real-time events.

## Enabling GraphQL

GraphQL support requires the `graphql` feature flag:

```bash
# Build with GraphQL support
cargo build -p ws-server --features graphql

# Run with GraphQL enabled
cargo run -p ws-server --features graphql -- --enable-graphql --graphql-port 8891
```

## Endpoints

- **GraphQL Playground**: `http://localhost:8891/graphql` (GET)
- **GraphQL API**: `http://localhost:8891/graphql` (POST)
- **GraphQL Subscriptions**: `ws://localhost:8891/graphql/ws` (WebSocket)

## Schema

### Types

```graphql
# Cycle phases
enum Phase {
  SHADOW
  COMPOSTING
  LIMINAL
  NEGATIVE_CAPABILITY
  EROS
  CO_CREATION
  BEAUTY
  EMERGENT_PERSONHOOD
  KENOSIS
}

# Current cycle state
type CycleState {
  cycleNumber: Int!
  currentPhase: Phase!
  phaseStarted: String!      # ISO8601 timestamp
  cycleStarted: String!      # ISO8601 timestamp
  phaseDay: Int!             # Day within current phase (0-indexed)
  phaseDurationDays: Int!    # Total days in current phase
  isRunning: Boolean!
}

# Phase transition record
type PhaseTransition {
  fromPhase: Phase!
  toPhase: Phase!
  cycleNumber: Int!
  transitionedAt: String!    # ISO8601 timestamp
}

# Metrics for a phase
type PhaseMetrics {
  phase: Phase!
  activeAgents: Int!
  spectralK: Float!
  meanMetabolicTrust: Float!
  activeWounds: Int!
  compostingEntities: Int!
  liminalEntities: Int!
  entangledPairs: Int!
  heldUncertainties: Int!
}

# Phase change event (for subscriptions)
type PhaseChangeEvent {
  fromPhase: Phase!
  toPhase: Phase!
  cycleNumber: Int!
  timestamp: String!
}

# Cycle start event (for subscriptions)
type CycleStartEvent {
  cycleNumber: Int!
  startedAt: String!
}

# Generic protocol event (for subscriptions)
type ProtocolEvent {
  eventType: String!
  payload: String!          # JSON-encoded event data
  timestamp: String!
}
```

### Queries

```graphql
type Query {
  # Get current cycle state
  cycleState: CycleState!

  # Get current phase
  currentPhase: Phase!

  # Get current cycle number
  cycleNumber: Int!

  # Get transition history (most recent first)
  transitionHistory(limit: Int): [PhaseTransition!]!

  # Get metrics for a specific phase
  phaseMetrics(phase: Phase!): PhaseMetrics!

  # Check if an operation is permitted in current phase
  isOperationPermitted(operation: String!): Boolean!
}
```

### Subscriptions

```graphql
type Subscription {
  # Subscribe to phase change events
  onPhaseChange: PhaseChangeEvent!

  # Subscribe to cycle start events
  onCycleStart: CycleStartEvent!

  # Subscribe to all protocol events (with optional type filter)
  onEvent(eventType: String): ProtocolEvent!
}
```

## Example Queries

### Get Current State

```graphql
query GetCycleState {
  cycleState {
    cycleNumber
    currentPhase
    phaseStarted
    phaseDay
    phaseDurationDays
    isRunning
  }
}
```

Response:
```json
{
  "data": {
    "cycleState": {
      "cycleNumber": 1,
      "currentPhase": "SHADOW",
      "phaseStarted": "2024-01-15T00:00:00Z",
      "phaseDay": 0,
      "phaseDurationDays": 2,
      "isRunning": true
    }
  }
}
```

### Get Transition History

```graphql
query GetHistory {
  transitionHistory(limit: 5) {
    fromPhase
    toPhase
    cycleNumber
    transitionedAt
  }
}
```

### Get Phase Metrics

```graphql
query GetShadowMetrics {
  phaseMetrics(phase: SHADOW) {
    phase
    activeAgents
    spectralK
    meanMetabolicTrust
    activeWounds
  }
}
```

### Check Operation Permission

```graphql
query CanVote {
  isOperationPermitted(operation: "vote")
}
```

## Example Subscriptions

### Subscribe to Phase Changes

```graphql
subscription OnPhaseChange {
  onPhaseChange {
    fromPhase
    toPhase
    cycleNumber
    timestamp
  }
}
```

### Subscribe to All Events

```graphql
subscription OnAllEvents {
  onEvent {
    eventType
    payload
    timestamp
  }
}
```

### Subscribe to Specific Event Type

```graphql
subscription OnPhaseTransitions {
  onEvent(eventType: "PhaseTransitioned") {
    eventType
    payload
    timestamp
  }
}
```

## Event Types

The following event types can be subscribed to via `onEvent`:

### Metabolism Events
- `CompostingStarted`
- `NutrientExtracted`
- `CompostingCompleted`
- `WoundCreated`
- `WoundPhaseAdvanced`
- `RestitutionFulfilled`
- `ScarTissueFormed`
- `MetabolicTrustUpdated`
- `KenosisCommitted`
- `KenosisExecuted`

### Consciousness Events
- `TemporalKVectorUpdated`
- `FieldInterferenceDetected`
- `DreamStateChanged`
- `DreamProposalGenerated`
- `NetworkPhiComputed`

### Epistemic Events
- `ShadowSurfaced`
- `ClaimHeldInUncertainty`
- `ClaimReleasedFromUncertainty`
- `SilenceDetected`
- `BeautyScored`

### Relational Events
- `EntanglementFormed`
- `EntanglementDecayed`
- `AttractorFieldComputed`
- `LiminalTransitionStarted`
- `LiminalTransitionCompleted`
- `InterSpeciesRegistered`

### Structural Events
- `ResonanceAddressCreated`
- `FractalPatternReplicated`
- `MorphogeneticFieldUpdated`
- `TimeCrystalPeriodStarted`
- `MycelialTaskDistributed`
- `MycelialTaskCompleted`

### Cycle Events
- `PhaseTransitioned`
- `CycleStarted`

## Client Libraries

### JavaScript/TypeScript

Using `graphql-ws`:

```typescript
import { createClient } from 'graphql-ws';

const client = createClient({
  url: 'ws://localhost:8891/graphql/ws',
});

// Subscribe to phase changes
const subscription = client.subscribe(
  {
    query: `
      subscription {
        onPhaseChange {
          fromPhase
          toPhase
          cycleNumber
          timestamp
        }
      }
    `,
  },
  {
    next: (data) => console.log('Phase changed:', data),
    error: (err) => console.error('Subscription error:', err),
    complete: () => console.log('Subscription complete'),
  }
);

// Cleanup
subscription.unsubscribe();
```

### Python

Using `gql` with `websockets`:

```python
from gql import Client, gql
from gql.transport.websockets import WebsocketsTransport

transport = WebsocketsTransport(url='ws://localhost:8891/graphql/ws')

client = Client(
    transport=transport,
    fetch_schema_from_transport=True,
)

# Query
query = gql("""
    query {
        cycleState {
            cycleNumber
            currentPhase
            phaseDay
        }
    }
""")

result = client.execute(query)
print(result)

# Subscription
subscription = gql("""
    subscription {
        onPhaseChange {
            fromPhase
            toPhase
            timestamp
        }
    }
""")

for result in client.subscribe(subscription):
    print(result)
```

## CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--enable-graphql` | false | Enable GraphQL API |
| `--graphql-port` | 8891 | Port for GraphQL server |
| `--no-graphql-playground` | false | Disable GraphQL Playground |

## Security Considerations

1. **Authentication**: The GraphQL API inherits authentication from the main server configuration. Use `--require-auth` and `--api-keys` to secure access.

2. **Rate Limiting**: GraphQL queries count towards the rate limit configured with `--rate-limit`.

3. **Introspection**: Introspection is enabled by default for development. Consider disabling in production by modifying the GraphQL configuration.

4. **Query Complexity**: Currently no query complexity limits are enforced. Consider adding limits for production deployments.
