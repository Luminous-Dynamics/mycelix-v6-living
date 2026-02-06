# Mycelix Go SDK

Go SDK for the Mycelix Living Protocol WebSocket server. This SDK provides client access to metabolism cycle state and real-time events.

## Installation

```bash
go get github.com/mycelix/mycelix-go
```

Or add to your `go.mod`:

```
require github.com/mycelix/mycelix-go v0.1.0
```

## Quick Start

### Connect and Query State

```go
package main

import (
    "context"
    "fmt"
    "log"

    mycelix "github.com/mycelix/mycelix-go"
)

func main() {
    // Connect to the server
    client, err := mycelix.Connect("ws://localhost:8888")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    ctx := context.Background()

    // Get current cycle state
    state, err := client.GetCurrentState(ctx)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Cycle: %d\n", state.CycleNumber)
    fmt.Printf("Phase: %s\n", state.CurrentPhase)
    fmt.Printf("Phase Day: %d\n", state.PhaseDay)

    // Get phase-specific metrics
    metrics, err := client.GetPhaseMetrics(ctx, state.CurrentPhase)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Active Agents: %d\n", metrics.ActiveAgents)
    fmt.Printf("Metabolic Trust: %.2f\n", metrics.MeanMetabolicTrust)

    // Check if an operation is permitted
    canVote, err := client.IsOperationPermitted(ctx, "vote")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Voting permitted: %t\n", canVote)
}
```

### Subscribe to Events

```go
package main

import (
    "fmt"
    "log"

    mycelix "github.com/mycelix/mycelix-go"
)

func main() {
    client, err := mycelix.Connect("ws://localhost:8888")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Subscribe to all events
    events := client.Subscribe()

    for event := range events.Events() {
        fmt.Printf("Event: %s\n", event.Type)
    }
}
```

### Subscribe to Specific Event Types

```go
package main

import (
    "fmt"
    "log"

    mycelix "github.com/mycelix/mycelix-go"
)

func main() {
    client, err := mycelix.Connect("ws://localhost:8888")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Subscribe only to phase transitions
    events := client.SubscribePhaseTransitions()

    for event := range events.Events() {
        transition := event.AsPhaseTransition()
        if transition != nil {
            fmt.Printf("Phase changed: %s -> %s\n",
                transition.From, transition.To)
        }
    }
}
```

### Custom Event Filters

```go
package main

import (
    "fmt"
    "log"

    mycelix "github.com/mycelix/mycelix-go"
)

func main() {
    client, err := mycelix.Connect("ws://localhost:8888")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Subscribe with custom filter
    events := client.SubscribeFiltered(
        mycelix.EventTypeFilter("PhaseTransitioned", "CycleStarted"),
    )

    for event := range events.Events() {
        fmt.Printf("Important event: %s\n", event.Type)
    }
}
```

### Use REST API

The SDK also supports the REST API endpoints (requires `--enable-rest` on the server):

```go
package main

import (
    "context"
    "fmt"
    "log"

    mycelix "github.com/mycelix/mycelix-go"
)

func main() {
    client := &mycelix.LivingProtocolClient{
        Config: mycelix.ClientConfig{
            URL:     "ws://localhost:8888",
            RESTURL: "http://localhost:8889",
        },
    }

    ctx := context.Background()

    // REST endpoints (no WebSocket connection needed)
    state, err := client.RESTGetState(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Phase: %s\n", state.CurrentPhase)

    history, err := client.RESTGetHistory(ctx)
    if err != nil {
        log.Fatal(err)
    }
    for _, t := range history {
        fmt.Printf("  %s -> %s\n", t.From, t.To)
    }
}
```

## Configuration

Configure the client with custom settings:

```go
config := mycelix.ClientConfig{
    URL:                  "ws://localhost:8888",
    RESTURL:              "http://localhost:8889", // Optional
    Reconnect:            true,                    // Auto-reconnect
    ReconnectDelay:       time.Second,             // Initial delay
    MaxReconnectAttempts: 10,                      // Max attempts
    PingInterval:         30 * time.Second,        // Ping interval
    RequestTimeout:       10 * time.Second,        // RPC timeout
}

client, err := mycelix.ConnectWithConfig(config)
```

## Cycle Phases

The Living Protocol follows a 28-day metabolism cycle:

| Phase | Duration | Description |
|-------|----------|-------------|
| Shadow | 2 days | Suppression detection, dissent surfaces |
| Composting | 5 days | Failed entities decomposed, nutrients extracted |
| Liminal | 3 days | Transitioning entities in threshold state |
| NegativeCapability | 3 days | Open questions held, voting blocked |
| Eros | 4 days | Attractor fields computed |
| CoCreation | 7 days | Standard consensus, entangled pairs form |
| Beauty | 2 days | Proposals scored on aesthetic criteria |
| EmergentPersonhood | 1 day | Network self-measurement |
| Kenosis | 1 day | Voluntary reputation release |

Access phase information:

```go
phase := mycelix.PhaseCoCreation
fmt.Printf("Duration: %d days\n", phase.DurationDays())
fmt.Printf("Next phase: %s\n", phase.Next())
fmt.Printf("Previous phase: %s\n", phase.Prev())

// Total cycle length
total := mycelix.TotalCycleDays() // 28
```

## Event Types

The SDK handles various event types:

- `PhaseTransitioned` - Cycle phase changed
- `CycleStarted` - New 28-day cycle began
- `WoundCreated`, `WoundPhaseAdvanced` - Wound healing events
- `KenosisCommitted`, `KenosisExecuted` - Reputation release events
- `EntanglementFormed`, `EntanglementDecayed` - Agent entanglement events
- And many more...

## Error Handling

```go
import mycelix "github.com/mycelix/mycelix-go"

client, err := mycelix.Connect("ws://localhost:8888")
if err != nil {
    log.Fatalf("Failed to connect: %v", err)
}
defer client.Close()

ctx := context.Background()
state, err := client.GetCurrentState(ctx)
if err != nil {
    if rpcErr, ok := err.(*mycelix.RpcError); ok {
        log.Printf("RPC error %d: %s", rpcErr.Code, rpcErr.Message)
    } else {
        log.Printf("Error: %v", err)
    }
}
```

## Testing

Run tests:

```bash
cd sdk/go
go test -v ./...
```

## Thread Safety

The client is safe for concurrent use from multiple goroutines. All methods that access shared state use proper synchronization.

## License

MIT License
