---
sidebar_position: 3
title: GraphQL Schema
---

# GraphQL Schema

Query and mutate Mycelix using GraphQL.

## Endpoint

```
POST /graphql
GET  /graphql (for playground)
```

## Authentication

Include credentials in request headers:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJhbG..." \
  -d '{"query": "{ cycle { phase day } }"}' \
  https://mycelix.example.com/graphql
```

## Schema Overview

```graphql
type Query {
  # Cycle queries
  cycle: Cycle!
  cycleHistory(limit: Int, offset: Int): CycleHistoryConnection!

  # Primitive queries
  primitive(id: ID!): Primitive
  primitives(
    type: PrimitiveType
    status: PrimitiveStatus
    first: Int
    after: String
  ): PrimitiveConnection!

  # Storage queries
  store(key: String!, store: StoreType): StoreEntry
  storeKeys(prefix: String, store: StoreType, first: Int, after: String): KeyConnection!

  # Cluster queries
  cluster: Cluster!
  node(id: ID!): Node
}

type Mutation {
  # Primitive mutations
  createPrimitive(input: CreatePrimitiveInput!): Primitive!
  updatePrimitive(id: ID!, input: UpdatePrimitiveInput!): Primitive!
  deletePrimitive(id: ID!): DeleteResult!
  invokePrimitive(id: ID!, payload: JSON!): InvokeResult!
  pausePrimitive(id: ID!): Primitive!
  resumePrimitive(id: ID!): Primitive!

  # Storage mutations
  setStore(key: String!, value: JSON!, store: StoreType, ttl: String): StoreEntry!
  deleteStore(key: String!, store: StoreType): DeleteResult!
}

type Subscription {
  # Cycle subscriptions
  cyclePhaseChanged: PhaseChangeEvent!
  cycleDayChanged: DayChangeEvent!

  # Primitive subscriptions
  primitiveEmitted(id: ID!): EmitEvent!
  primitiveStateChanged(id: ID!): StateChangeEvent!

  # Cluster subscriptions
  nodeStatusChanged: NodeStatusEvent!
}
```

## Types

### Cycle Types

```graphql
type Cycle {
  phase: Phase!
  day: Int!
  cycleNumber: Int!
  progress: Float!
  nextPhase: Phase!
  daysUntilNextPhase: Int!
  startDate: Date!
  currentCycleStart: Date!
  timestamp: DateTime!
}

enum Phase {
  Dawn
  Surge
  Settle
  Rest
}

type CycleHistory {
  number: Int!
  startDate: Date!
  endDate: Date
  status: CycleStatus!
  summary: CycleSummary
}

enum CycleStatus {
  ACTIVE
  COMPLETED
}

type CycleSummary {
  totalEvents: Int!
  peakThroughput: Int!
  errorRate: Float!
  avgLatency: Float!
}

type CycleHistoryConnection {
  edges: [CycleHistoryEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}
```

### Primitive Types

```graphql
enum PrimitiveType {
  PULSE
  SIGNAL
  ECHO
  THREAD
  WEAVE
  MESH
  SPORE
  BLOOM
  FRUIT
  ROOT
  MYCELIUM
  ARCHIVE
  STREAM
  POOL
  GATE
  SENSE
  DREAM
  WAKE
  CYCLE
  PHASE
  RHYTHM
}

enum PrimitiveStatus {
  ACTIVE
  PAUSED
  ERROR
  INITIALIZING
}

type Primitive {
  id: ID!
  type: PrimitiveType!
  name: String!
  status: PrimitiveStatus!
  config: JSON!
  stats: PrimitiveStats!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type PrimitiveStats {
  invocationCount: Int!
  errorCount: Int!
  lastInvocation: DateTime
  avgDuration: Float!
  successRate: Float!
}

type PrimitiveConnection {
  edges: [PrimitiveEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type PrimitiveEdge {
  node: Primitive!
  cursor: String!
}

input CreatePrimitiveInput {
  type: PrimitiveType!
  name: String!
  config: JSON!
}

input UpdatePrimitiveInput {
  name: String
  config: JSON
}

type InvokeResult {
  result: JSON!
  duration: Int!
  phase: Phase!
  timestamp: DateTime!
}
```

### Storage Types

```graphql
enum StoreType {
  ROOT
  MYCELIUM
  ARCHIVE
}

type StoreEntry {
  key: String!
  value: JSON!
  metadata: StoreMetadata!
}

type StoreMetadata {
  version: Int!
  createdAt: DateTime!
  updatedAt: DateTime!
  ttl: DateTime
  size: Int!
}

type KeyConnection {
  edges: [KeyEdge!]!
  pageInfo: PageInfo!
}

type KeyEdge {
  node: String!
  cursor: String!
}
```

### Cluster Types

```graphql
type Cluster {
  id: String!
  leader: Node
  nodes: [Node!]!
  healthyCount: Int!
  totalCount: Int!
  status: ClusterStatus!
}

enum ClusterStatus {
  HEALTHY
  DEGRADED
  UNHEALTHY
}

type Node {
  id: ID!
  address: String!
  status: NodeStatus!
  role: NodeRole!
  phase: Phase!
  uptime: Int!
  version: String!
  primitives: NodePrimitives!
  metrics: NodeMetrics!
}

enum NodeStatus {
  HEALTHY
  UNHEALTHY
  UNKNOWN
}

enum NodeRole {
  LEADER
  FOLLOWER
}

type NodePrimitives {
  active: Int!
  paused: Int!
  error: Int!
}

type NodeMetrics {
  cpu: Float!
  memory: Float!
  disk: Float!
  connections: Int!
  requestsPerSecond: Float!
}
```

### Event Types

```graphql
type PhaseChangeEvent {
  from: Phase!
  to: Phase!
  day: Int!
  cycleNumber: Int!
  timestamp: DateTime!
}

type DayChangeEvent {
  day: Int!
  phase: Phase!
  cycleNumber: Int!
  timestamp: DateTime!
}

type EmitEvent {
  primitiveId: ID!
  data: JSON!
  timestamp: DateTime!
}

type StateChangeEvent {
  primitiveId: ID!
  from: PrimitiveStatus!
  to: PrimitiveStatus!
  timestamp: DateTime!
}

type NodeStatusEvent {
  nodeId: ID!
  from: NodeStatus!
  to: NodeStatus!
  timestamp: DateTime!
}
```

### Common Types

```graphql
type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

type DeleteResult {
  success: Boolean!
  id: ID!
}

scalar JSON
scalar Date
scalar DateTime
```

## Query Examples

### Get Cycle Status

```graphql
query GetCycle {
  cycle {
    phase
    day
    cycleNumber
    progress
    nextPhase
    daysUntilNextPhase
  }
}
```

### List Primitives

```graphql
query ListPrimitives($type: PrimitiveType, $first: Int) {
  primitives(type: $type, first: $first) {
    edges {
      node {
        id
        type
        name
        status
        stats {
          invocationCount
          successRate
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
  }
}
```

### Get Primitive with Stats

```graphql
query GetPrimitive($id: ID!) {
  primitive(id: $id) {
    id
    type
    name
    status
    config
    stats {
      invocationCount
      errorCount
      avgDuration
      successRate
      lastInvocation
    }
    createdAt
    updatedAt
  }
}
```

### Cluster Overview

```graphql
query ClusterOverview {
  cluster {
    id
    status
    healthyCount
    totalCount
    leader {
      id
      address
    }
    nodes {
      id
      status
      role
      phase
      metrics {
        cpu
        memory
        connections
      }
    }
  }
}
```

## Mutation Examples

### Create Primitive

```graphql
mutation CreatePulse($input: CreatePrimitiveInput!) {
  createPrimitive(input: $input) {
    id
    type
    name
    status
  }
}

# Variables
{
  "input": {
    "type": "PULSE",
    "name": "heartbeat",
    "config": {
      "interval": {
        "Dawn": "10s",
        "Surge": "1s",
        "Settle": "5s",
        "Rest": "30s"
      }
    }
  }
}
```

### Invoke Primitive

```graphql
mutation InvokePrimitive($id: ID!, $payload: JSON!) {
  invokePrimitive(id: $id, payload: $payload) {
    result
    duration
    phase
  }
}

# Variables
{
  "id": "thread-abc123",
  "payload": {
    "action": "process",
    "data": { "key": "value" }
  }
}
```

### Update Store

```graphql
mutation SetStore($key: String!, $value: JSON!) {
  setStore(key: $key, value: $value, store: ROOT, ttl: "24h") {
    key
    metadata {
      version
      updatedAt
    }
  }
}
```

## Subscription Examples

### Subscribe to Phase Changes

```graphql
subscription OnPhaseChange {
  cyclePhaseChanged {
    from
    to
    day
    cycleNumber
    timestamp
  }
}
```

### Subscribe to Primitive Emissions

```graphql
subscription OnPulseEmit($id: ID!) {
  primitiveEmitted(id: $id) {
    primitiveId
    data
    timestamp
  }
}
```

## Directives

### @phaseRestrict

Restrict field access by phase:

```graphql
type Mutation {
  # Only allowed during Dawn or Rest
  deployPrimitive(id: ID!): Primitive!
    @phaseRestrict(phases: [Dawn, Rest])
}
```

### @rateLimit

Apply rate limiting:

```graphql
type Query {
  # Limited to 100 requests per minute
  primitives: PrimitiveConnection!
    @rateLimit(limit: 100, window: "1m")
}
```

## Complexity Limits

GraphQL queries are limited by complexity:

| Phase | Max Complexity |
|-------|----------------|
| Dawn | 5000 |
| Surge | 10000 |
| Settle | 7500 |
| Rest | 2500 |

Check complexity in response headers:

```
X-GraphQL-Complexity: 150
X-GraphQL-Complexity-Limit: 10000
```

## Error Handling

GraphQL errors follow the standard format:

```json
{
  "errors": [
    {
      "message": "Primitive not found",
      "locations": [{ "line": 2, "column": 3 }],
      "path": ["primitive"],
      "extensions": {
        "code": "NOT_FOUND",
        "phase": "Surge"
      }
    }
  ],
  "data": null
}
```

## Next Steps

- [WebSocket RPC](./websocket) - Real-time API
- [REST API](./rest) - HTTP REST API
