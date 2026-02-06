---
sidebar_position: 2
title: Python SDK
---

# Python SDK

Async-first Python SDK for Mycelix.

## Installation

```bash
pip install mycelix
# or with uv
uv add mycelix
# or with poetry
poetry add mycelix
```

## Quick Start

```python
import asyncio
from mycelix import MycelixClient

async def main():
    # Create and connect
    async with MycelixClient(
        url="wss://mycelix.example.com/ws",
        api_key="mk_prod_xxx"
    ) as client:
        # Get cycle status
        cycle = await client.cycle.status()
        print(f"Phase: {cycle.phase}, Day: {cycle.day}")

        # Invoke a primitive
        result = await client.primitives.invoke(
            "thread-abc123",
            {"action": "process", "data": {"key": "value"}}
        )
        print(result)

asyncio.run(main())
```

## Client Configuration

```python
from mycelix import MycelixClient, ClientConfig

config = ClientConfig(
    # Connection
    url="wss://mycelix.example.com/ws",

    # Authentication
    api_key="mk_prod_xxx",
    # or
    jwt="eyJhbG...",
    # or
    auth=lambda: {"type": "bearer", "token": get_token()},

    # Connection options
    auto_connect=True,
    reconnect=True,
    reconnect_delay=1.0,
    max_reconnect_attempts=10,

    # Timeouts
    connect_timeout=10.0,
    request_timeout=30.0,
)

client = MycelixClient(config)
```

## Cycle API

### Get Status

```python
status = await client.cycle.status()

print(status.phase)              # Phase.DAWN | Phase.SURGE | Phase.SETTLE | Phase.REST
print(status.day)                # 1-28
print(status.cycle_number)       # Current cycle number
print(status.progress)           # 0.0 - 1.0
print(status.next_phase)         # Next phase
print(status.days_until_next_phase)
```

### Subscribe to Changes

```python
# Using decorators
@client.on("cycle.phase_change")
async def on_phase_change(event):
    print(f"Phase: {event.from_phase} -> {event.to_phase}")

@client.on("cycle.day_change")
async def on_day_change(event):
    print(f"Day {event.day} of {event.phase}")

# Or using context manager
async with client.cycle.subscribe() as events:
    async for event in events:
        if event.type == "phase_change":
            print(f"Phase changed to {event.to_phase}")
```

### Phase-Conditional Logic

```python
from mycelix import Phase

# Check current phase
if client.cycle.current_phase == Phase.SURGE:
    # High-throughput operations
    pass

# Wait for specific phase
await client.cycle.wait_for_phase(Phase.DAWN)

# Execute only in specific phases
async with client.cycle.during_phase(Phase.SETTLE):
    await run_analytics()

# Phase-aware decorator
@client.phase_aware(allowed=[Phase.DAWN, Phase.REST])
async def deploy():
    # Only runs during Dawn or Rest
    pass
```

## Primitives API

### List Primitives

```python
from mycelix import PrimitiveType, PrimitiveStatus

result = await client.primitives.list(
    type=PrimitiveType.PULSE,
    status=PrimitiveStatus.ACTIVE,
    limit=50
)

for p in result.primitives:
    print(f"{p.name} ({p.type}): {p.status}")
```

### Get Primitive

```python
pulse = await client.primitives.get("pulse-abc123")

print(pulse.config)
print(pulse.stats.invocation_count)
print(pulse.stats.avg_duration)
```

### Create Primitive

```python
from mycelix import PrimitiveType

primitive = await client.primitives.create(
    type=PrimitiveType.PULSE,
    name="metrics-collector",
    config={
        "interval": {
            "Dawn": "10s",
            "Surge": "1s",
            "Settle": "5s",
            "Rest": "30s",
        },
        "emit": {
            "type": "metrics",
            "include": ["cpu", "memory"],
        },
    }
)

print(f"Created: {primitive.id}")
```

### Invoke Primitive

```python
# Simple invocation
result = await client.primitives.invoke(
    "thread-abc123",
    {"action": "process", "data": {"key": "value"}}
)

# With options
result = await client.primitives.invoke(
    "thread-abc123",
    payload,
    timeout=5.0,
    priority="high"
)
```

### Subscribe to Primitive Events

```python
# Using async iterator
async with client.primitives.subscribe("pulse-abc123") as events:
    async for event in events:
        if event.type == "emit":
            print(f"Pulse emitted: {event.data}")
        elif event.type == "error":
            print(f"Error: {event.error}")

# Using callback
async def on_emit(data):
    print(f"Pulse emitted: {data}")

unsubscribe = await client.primitives.subscribe(
    "pulse-abc123",
    on_emit=on_emit
)

# Later
await unsubscribe()
```

### Pause/Resume

```python
await client.primitives.pause("pulse-abc123")
await client.primitives.resume("pulse-abc123")
```

### Delete

```python
await client.primitives.delete("pulse-abc123")
```

## Storage API

### Get/Set Values

```python
from mycelix import StoreType

# Set value
await client.store.set(
    "user:123",
    {"name": "Alice", "role": "admin"},
    store=StoreType.ROOT,
    ttl="24h"
)

# Get value
user = await client.store.get("user:123", store=StoreType.ROOT)

# Get with type hint
from mycelix import TypedStore

users: TypedStore[User] = client.store.typed(User)
user = await users.get("user:123")  # Returns User

# Delete value
await client.store.delete("user:123")
```

### List Keys

```python
result = await client.store.keys(
    prefix="user:",
    store=StoreType.ROOT,
    limit=100
)

for key in result.keys:
    print(key)

# Iterate through all keys
async for key in client.store.keys_iter(prefix="user:"):
    print(key)
```

### Transactions

```python
async with client.store.transaction() as tx:
    balance = await tx.get("account:123:balance")
    await tx.set("account:123:balance", balance - 100)
    await tx.set("account:456:balance", balance + 100)
```

## Cluster API

### Get Status

```python
cluster = await client.cluster.status()

print(f"Nodes: {cluster.healthy}/{cluster.total}")
print(f"Leader: {cluster.leader.id if cluster.leader else 'None'}")

for node in cluster.nodes:
    print(f"{node.id}: {node.status} ({node.role})")
```

### Get Node Details

```python
node = await client.cluster.node("node-1")

print(f"CPU: {node.metrics.cpu}")
print(f"Memory: {node.metrics.memory}")
print(f"Connections: {node.metrics.connections}")
```

### Subscribe to Cluster Events

```python
@client.on("cluster.node_status_change")
async def on_node_status(event):
    print(f"Node {event.node_id}: {event.from_status} -> {event.to_status}")
```

## Type Hints

The SDK is fully typed with Python type hints:

```python
from mycelix import (
    Phase,
    Cycle,
    Primitive,
    PrimitiveType,
    PrimitiveStatus,
    PrimitiveConfig,
    Node,
    Cluster,
    StoreEntry,
)

# Type hints work with IDE autocomplete
async def process_cycle(cycle: Cycle) -> None:
    if cycle.phase == Phase.SURGE:
        print("High throughput mode")
```

### Pydantic Models

All response types are Pydantic models:

```python
from mycelix import Primitive

primitive = await client.primitives.get("pulse-abc123")

# Access as dict
data = primitive.model_dump()

# JSON serialization
json_str = primitive.model_dump_json()

# Validation
primitive = Primitive.model_validate(data)
```

## Error Handling

```python
from mycelix import (
    MycelixError,
    NotFoundError,
    PhaseRestrictedError,
    RateLimitedError,
    AuthenticationError,
)

try:
    await client.primitives.invoke("thread-abc123", payload)
except NotFoundError:
    print("Primitive not found")
except PhaseRestrictedError as e:
    print(f"Not allowed in {e.phase} phase")
except RateLimitedError as e:
    print(f"Retry after {e.retry_after}s")
    await asyncio.sleep(e.retry_after)
except MycelixError as e:
    print(f"Error: {e.code} - {e.message}")
```

## Connection Management

```python
# Manual connection control
await client.connect()
await client.disconnect()

# Context manager (recommended)
async with MycelixClient(url="...", api_key="...") as client:
    # Connected
    pass
# Disconnected

# Connection events
@client.on("connect")
async def on_connect():
    print("Connected")

@client.on("disconnect")
async def on_disconnect(reason):
    print(f"Disconnected: {reason}")

# Check connection state
print(client.connected)  # bool
print(client.state)      # "connecting" | "connected" | "disconnected"
```

## Synchronous API

For environments that don't support async:

```python
from mycelix.sync import MycelixClient

# Synchronous client
client = MycelixClient(url="...", api_key="...")
client.connect()

# Sync methods
cycle = client.cycle.status()
print(f"Phase: {cycle.phase}")

# Clean up
client.disconnect()
```

## Django Integration

```python
# settings.py
MYCELIX = {
    "URL": "wss://mycelix.example.com/ws",
    "API_KEY": os.environ["MYCELIX_API_KEY"],
}

# mycelix_client.py
from django.conf import settings
from mycelix.django import get_client

async def get_cycle():
    client = await get_client()
    return await client.cycle.status()

# views.py
from django.http import JsonResponse
from .mycelix_client import get_cycle

async def cycle_view(request):
    cycle = await get_cycle()
    return JsonResponse({"phase": cycle.phase, "day": cycle.day})
```

## FastAPI Integration

```python
from fastapi import FastAPI, Depends
from mycelix.fastapi import MycelixDependency, get_client

app = FastAPI()
mycelix = MycelixDependency(
    url="wss://mycelix.example.com/ws",
    api_key="mk_xxx"
)

@app.on_event("startup")
async def startup():
    await mycelix.connect()

@app.on_event("shutdown")
async def shutdown():
    await mycelix.disconnect()

@app.get("/cycle")
async def get_cycle(client = Depends(get_client)):
    return await client.cycle.status()
```

## Testing

```python
from mycelix.testing import MockClient, mock_cycle, mock_primitive

# Create mock client
client = MockClient(
    cycle=mock_cycle(phase="Surge", day=10),
    primitives=[
        mock_primitive(id="pulse-1", type="pulse", name="test"),
    ],
)

# Use in tests
async def test_cycle():
    status = await client.cycle.status()
    assert status.phase == "Surge"

# Pytest fixture
import pytest
from mycelix.testing import mycelix_mock

@pytest.fixture
def client():
    return mycelix_mock()

async def test_with_fixture(client):
    status = await client.cycle.status()
    assert status.phase == "Dawn"
```

## Next Steps

- [Go SDK](./go) - Go client
- [TypeScript SDK](./typescript) - TypeScript/JavaScript client
- [WebSocket API](../api/websocket) - Protocol reference
