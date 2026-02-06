---
sidebar_position: 3
title: Go SDK
---

# Go SDK

High-performance Go SDK for Mycelix.

## Installation

```bash
go get github.com/mycelix/mycelix-go
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    "github.com/mycelix/mycelix-go"
)

func main() {
    // Create client
    client, err := mycelix.NewClient(mycelix.Config{
        URL:    "wss://mycelix.example.com/ws",
        APIKey: "mk_prod_xxx",
    })
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Connect
    ctx := context.Background()
    if err := client.Connect(ctx); err != nil {
        log.Fatal(err)
    }

    // Get cycle status
    cycle, err := client.Cycle().Status(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Phase: %s, Day: %d\n", cycle.Phase, cycle.Day)

    // Invoke a primitive
    result, err := client.Primitives().Invoke(ctx, "thread-abc123", map[string]any{
        "action": "process",
        "data":   map[string]any{"key": "value"},
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Println(result)
}
```

## Client Configuration

```go
import "github.com/mycelix/mycelix-go"

config := mycelix.Config{
    // Connection
    URL: "wss://mycelix.example.com/ws",

    // Authentication
    APIKey: "mk_prod_xxx",
    // or
    JWT: "eyJhbG...",
    // or
    AuthFunc: func(ctx context.Context) (*mycelix.Auth, error) {
        token, err := getToken(ctx)
        return &mycelix.Auth{Type: "bearer", Token: token}, err
    },

    // Connection options
    AutoConnect:          true,
    Reconnect:            true,
    ReconnectDelay:       time.Second,
    MaxReconnectAttempts: 10,

    // Timeouts
    ConnectTimeout: 10 * time.Second,
    RequestTimeout: 30 * time.Second,

    // TLS
    TLSConfig: &tls.Config{
        // Custom TLS settings
    },

    // Logging
    Logger: slog.Default(),
}

client, err := mycelix.NewClient(config)
```

## Cycle API

### Get Status

```go
ctx := context.Background()
status, err := client.Cycle().Status(ctx)
if err != nil {
    log.Fatal(err)
}

fmt.Println(status.Phase)              // mycelix.PhaseDawn | PhaseSurge | PhaseSettle | PhaseRest
fmt.Println(status.Day)                // 1-28
fmt.Println(status.CycleNumber)        // Current cycle number
fmt.Println(status.Progress)           // 0.0 - 1.0
fmt.Println(status.NextPhase)          // Next phase
fmt.Println(status.DaysUntilNextPhase) // Days remaining
```

### Subscribe to Changes

```go
// Subscribe to phase changes
phaseChanges := make(chan *mycelix.PhaseChangeEvent)
unsubscribe, err := client.Cycle().OnPhaseChange(ctx, func(event *mycelix.PhaseChangeEvent) {
    fmt.Printf("Phase: %s -> %s\n", event.From, event.To)
})
if err != nil {
    log.Fatal(err)
}
defer unsubscribe()

// Or use channel
events, unsubscribe, err := client.Cycle().Subscribe(ctx)
if err != nil {
    log.Fatal(err)
}
defer unsubscribe()

for event := range events {
    switch e := event.(type) {
    case *mycelix.PhaseChangeEvent:
        fmt.Printf("Phase changed to %s\n", e.To)
    case *mycelix.DayChangeEvent:
        fmt.Printf("Day %d\n", e.Day)
    }
}
```

### Phase-Conditional Logic

```go
// Check current phase
if client.Cycle().CurrentPhase() == mycelix.PhaseSurge {
    // High-throughput operations
}

// Wait for specific phase
err := client.Cycle().WaitForPhase(ctx, mycelix.PhaseDawn)

// Execute only in specific phases
err := client.Cycle().DuringPhase(ctx, mycelix.PhaseSettle, func(ctx context.Context) error {
    return runAnalytics(ctx)
})
```

## Primitives API

### List Primitives

```go
result, err := client.Primitives().List(ctx, &mycelix.ListOptions{
    Type:   mycelix.PrimitiveTypePulse,
    Status: mycelix.StatusActive,
    Limit:  50,
})
if err != nil {
    log.Fatal(err)
}

for _, p := range result.Primitives {
    fmt.Printf("%s (%s): %s\n", p.Name, p.Type, p.Status)
}
```

### Get Primitive

```go
pulse, err := client.Primitives().Get(ctx, "pulse-abc123")
if err != nil {
    log.Fatal(err)
}

fmt.Println(pulse.Config)
fmt.Println(pulse.Stats.InvocationCount)
fmt.Println(pulse.Stats.AvgDuration)
```

### Create Primitive

```go
primitive, err := client.Primitives().Create(ctx, &mycelix.CreatePrimitiveInput{
    Type: mycelix.PrimitiveTypePulse,
    Name: "metrics-collector",
    Config: map[string]any{
        "interval": map[string]string{
            "Dawn":   "10s",
            "Surge":  "1s",
            "Settle": "5s",
            "Rest":   "30s",
        },
        "emit": map[string]any{
            "type":    "metrics",
            "include": []string{"cpu", "memory"},
        },
    },
})
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Created: %s\n", primitive.ID)
```

### Invoke Primitive

```go
// Simple invocation
result, err := client.Primitives().Invoke(ctx, "thread-abc123", map[string]any{
    "action": "process",
    "data":   map[string]any{"key": "value"},
})

// With options
result, err := client.Primitives().InvokeWithOptions(ctx, "thread-abc123", payload, &mycelix.InvokeOptions{
    Timeout:  5 * time.Second,
    Priority: "high",
})
```

### Subscribe to Primitive Events

```go
// Using callback
unsubscribe, err := client.Primitives().Subscribe(ctx, "pulse-abc123", &mycelix.SubscribeHandlers{
    OnEmit: func(data any) {
        fmt.Printf("Pulse emitted: %v\n", data)
    },
    OnError: func(err error) {
        fmt.Printf("Error: %v\n", err)
    },
    OnStateChange: func(from, to mycelix.PrimitiveStatus) {
        fmt.Printf("State: %s -> %s\n", from, to)
    },
})
if err != nil {
    log.Fatal(err)
}
defer unsubscribe()

// Using channel
events, unsubscribe, err := client.Primitives().SubscribeChan(ctx, "pulse-abc123")
if err != nil {
    log.Fatal(err)
}
defer unsubscribe()

for event := range events {
    switch e := event.(type) {
    case *mycelix.EmitEvent:
        fmt.Printf("Emitted: %v\n", e.Data)
    case *mycelix.ErrorEvent:
        fmt.Printf("Error: %v\n", e.Error)
    }
}
```

### Pause/Resume

```go
err := client.Primitives().Pause(ctx, "pulse-abc123")
err = client.Primitives().Resume(ctx, "pulse-abc123")
```

### Delete

```go
err := client.Primitives().Delete(ctx, "pulse-abc123")
```

## Storage API

### Get/Set Values

```go
// Set value
err := client.Store().Set(ctx, "user:123", User{
    Name: "Alice",
    Role: "admin",
}, &mycelix.SetOptions{
    Store: mycelix.StoreRoot,
    TTL:   24 * time.Hour,
})

// Get value
var user User
err = client.Store().Get(ctx, "user:123", &user, &mycelix.GetOptions{
    Store: mycelix.StoreRoot,
})

// Delete value
err = client.Store().Delete(ctx, "user:123", nil)
```

### List Keys

```go
result, err := client.Store().Keys(ctx, &mycelix.KeysOptions{
    Prefix: "user:",
    Store:  mycelix.StoreRoot,
    Limit:  100,
})

for _, key := range result.Keys {
    fmt.Println(key)
}

// Iterate through all keys
iter := client.Store().KeysIter(ctx, &mycelix.KeysOptions{Prefix: "user:"})
for iter.Next() {
    fmt.Println(iter.Key())
}
if err := iter.Err(); err != nil {
    log.Fatal(err)
}
```

### Transactions

```go
err := client.Store().Transaction(ctx, func(tx *mycelix.Tx) error {
    var balance int
    if err := tx.Get("account:123:balance", &balance); err != nil {
        return err
    }
    if err := tx.Set("account:123:balance", balance-100); err != nil {
        return err
    }
    return tx.Set("account:456:balance", balance+100)
})
```

## Cluster API

### Get Status

```go
cluster, err := client.Cluster().Status(ctx)
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Nodes: %d/%d\n", cluster.Healthy, cluster.Total)
if cluster.Leader != nil {
    fmt.Printf("Leader: %s\n", cluster.Leader.ID)
}

for _, node := range cluster.Nodes {
    fmt.Printf("%s: %s (%s)\n", node.ID, node.Status, node.Role)
}
```

### Get Node Details

```go
node, err := client.Cluster().Node(ctx, "node-1")
if err != nil {
    log.Fatal(err)
}

fmt.Printf("CPU: %.2f\n", node.Metrics.CPU)
fmt.Printf("Memory: %.2f\n", node.Metrics.Memory)
fmt.Printf("Connections: %d\n", node.Metrics.Connections)
```

### Subscribe to Cluster Events

```go
unsubscribe, err := client.Cluster().OnNodeStatusChange(ctx, func(event *mycelix.NodeStatusEvent) {
    fmt.Printf("Node %s: %s -> %s\n", event.NodeID, event.From, event.To)
})
defer unsubscribe()
```

## Types

### Core Types

```go
import "github.com/mycelix/mycelix-go"

type Phase string

const (
    PhaseDawn   Phase = "Dawn"
    PhaseSurge  Phase = "Surge"
    PhaseSettle Phase = "Settle"
    PhaseRest   Phase = "Rest"
)

type PrimitiveType string

const (
    PrimitiveTypePulse    PrimitiveType = "pulse"
    PrimitiveTypeThread   PrimitiveType = "thread"
    PrimitiveTypeStream   PrimitiveType = "stream"
    // ... etc
)

type Cycle struct {
    Phase              Phase     `json:"phase"`
    Day                int       `json:"day"`
    CycleNumber        int       `json:"cycleNumber"`
    Progress           float64   `json:"progress"`
    NextPhase          Phase     `json:"nextPhase"`
    DaysUntilNextPhase int       `json:"daysUntilNextPhase"`
    Timestamp          time.Time `json:"timestamp"`
}

type Primitive struct {
    ID        string          `json:"id"`
    Type      PrimitiveType   `json:"type"`
    Name      string          `json:"name"`
    Status    PrimitiveStatus `json:"status"`
    Config    map[string]any  `json:"config"`
    Stats     PrimitiveStats  `json:"stats"`
    CreatedAt time.Time       `json:"createdAt"`
    UpdatedAt time.Time       `json:"updatedAt"`
}
```

## Error Handling

```go
import "github.com/mycelix/mycelix-go"

result, err := client.Primitives().Invoke(ctx, "thread-abc123", payload)
if err != nil {
    var mycelixErr *mycelix.Error
    if errors.As(err, &mycelixErr) {
        switch mycelixErr.Code {
        case mycelix.ErrNotFound:
            fmt.Println("Primitive not found")
        case mycelix.ErrPhaseRestricted:
            fmt.Printf("Not allowed in %s phase\n", mycelixErr.Phase)
        case mycelix.ErrRateLimited:
            fmt.Printf("Retry after %v\n", mycelixErr.RetryAfter)
            time.Sleep(mycelixErr.RetryAfter)
        default:
            fmt.Printf("Error: %s\n", mycelixErr.Message)
        }
    }
    return
}
```

## Connection Management

```go
// Manual connection control
err := client.Connect(ctx)
err = client.Disconnect(ctx)
client.Close()

// Connection events
client.OnConnect(func() {
    fmt.Println("Connected")
})

client.OnDisconnect(func(reason string) {
    fmt.Printf("Disconnected: %s\n", reason)
})

client.OnReconnecting(func(attempt int) {
    fmt.Printf("Reconnecting: attempt %d\n", attempt)
})

// Check connection state
fmt.Println(client.Connected()) // bool
fmt.Println(client.State())     // "connecting" | "connected" | "disconnected"
```

## Concurrency Patterns

### Worker Pool

```go
func processWithWorkers(client *mycelix.Client, items []Item) error {
    ctx := context.Background()
    g, ctx := errgroup.WithContext(ctx)

    // Limit concurrency based on phase
    var semSize int
    switch client.Cycle().CurrentPhase() {
    case mycelix.PhaseSurge:
        semSize = 20
    case mycelix.PhaseSettle:
        semSize = 10
    default:
        semSize = 5
    }
    sem := make(chan struct{}, semSize)

    for _, item := range items {
        item := item
        g.Go(func() error {
            sem <- struct{}{}
            defer func() { <-sem }()

            _, err := client.Primitives().Invoke(ctx, "processor", item)
            return err
        })
    }

    return g.Wait()
}
```

### Graceful Shutdown

```go
func main() {
    client, _ := mycelix.NewClient(config)
    defer client.Close()

    ctx, cancel := context.WithCancel(context.Background())
    defer cancel()

    // Handle shutdown
    sigCh := make(chan os.Signal, 1)
    signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

    go func() {
        <-sigCh
        fmt.Println("Shutting down...")
        cancel()
    }()

    // Run until cancelled
    if err := run(ctx, client); err != nil && !errors.Is(err, context.Canceled) {
        log.Fatal(err)
    }
}
```

## Testing

```go
import "github.com/mycelix/mycelix-go/testing"

func TestMyFunction(t *testing.T) {
    // Create mock client
    client := testing.NewMockClient(
        testing.WithCycle(&mycelix.Cycle{
            Phase: mycelix.PhaseSurge,
            Day:   10,
        }),
        testing.WithPrimitives([]*mycelix.Primitive{
            {ID: "pulse-1", Type: mycelix.PrimitiveTypePulse, Name: "test"},
        }),
    )

    // Test your code
    cycle, err := client.Cycle().Status(context.Background())
    if err != nil {
        t.Fatal(err)
    }
    if cycle.Phase != mycelix.PhaseSurge {
        t.Errorf("expected Surge, got %s", cycle.Phase)
    }
}
```

## Next Steps

- [TypeScript SDK](./typescript) - TypeScript/JavaScript client
- [Python SDK](./python) - Python client
- [WebSocket API](../api/websocket) - Protocol reference
