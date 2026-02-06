# Mycelix Python SDK

Python SDK for the Mycelix Living Protocol WebSocket server. This SDK provides async-first access to metabolism cycle state and real-time events.

## Installation

```bash
pip install mycelix
```

Or install from source:

```bash
cd sdk/python
pip install -e ".[dev]"
```

## Quick Start

### Connect and Query State

```python
import asyncio
from mycelix import LivingProtocolClient, CyclePhase

async def main():
    async with LivingProtocolClient("ws://localhost:8888") as client:
        # Get current cycle state
        state = await client.get_current_state()
        print(f"Cycle: {state.cycle_number}")
        print(f"Phase: {state.current_phase}")
        print(f"Phase Day: {state.phase_day}")

        # Get phase-specific metrics
        metrics = await client.get_phase_metrics(state.current_phase)
        print(f"Active Agents: {metrics.active_agents}")
        print(f"Metabolic Trust: {metrics.mean_metabolic_trust:.2f}")

        # Check if an operation is permitted
        can_vote = await client.is_operation_permitted("vote")
        print(f"Voting permitted: {can_vote}")

asyncio.run(main())
```

### Subscribe to Events

```python
import asyncio
from mycelix import LivingProtocolClient

async def main():
    async with LivingProtocolClient("ws://localhost:8888") as client:
        # Subscribe to all events
        async for event in client.events():
            print(f"Event: {event.event_type}")
            print(f"Data: {event.data}")

asyncio.run(main())
```

### Subscribe to Specific Event Types

```python
import asyncio
from mycelix import LivingProtocolClient

async def main():
    async with LivingProtocolClient("ws://localhost:8888") as client:
        # Subscribe only to phase transitions
        sub = client.subscribe_phase_transitions()

        async for event in sub:
            transition = event.as_phase_transition()
            if transition:
                print(f"Phase changed: {transition.from_phase} -> {transition.to_phase}")

asyncio.run(main())
```

### Use REST API

The SDK also supports the REST API endpoints (requires `--enable-rest` on the server):

```python
import asyncio
from mycelix import LivingProtocolClient

async def main():
    client = LivingProtocolClient("ws://localhost:8888")

    # REST endpoints (no WebSocket connection needed)
    state = await client.rest_get_state()
    print(f"Phase: {state.current_phase}")

    history = await client.rest_get_history()
    for t in history:
        print(f"  {t.from_phase} -> {t.to_phase}")

asyncio.run(main())
```

## Configuration

Configure the client with custom settings:

```python
from mycelix import LivingProtocolClient
from mycelix.client import ClientConfig

config = ClientConfig(
    url="ws://localhost:8888",
    rest_url="http://localhost:8889",  # Optional: explicit REST URL
    reconnect=True,                     # Auto-reconnect on disconnect
    reconnect_delay=1.0,                # Initial reconnect delay (seconds)
    max_reconnect_attempts=10,          # Max reconnection attempts
    ping_interval=30.0,                 # WebSocket ping interval
    request_timeout=10.0,               # RPC request timeout
)

client = LivingProtocolClient(config)
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

```python
from mycelix import CyclePhase

phase = CyclePhase.CO_CREATION
print(f"Duration: {phase.duration_days} days")
print(f"Next phase: {phase.next()}")
print(f"Previous phase: {phase.prev()}")
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

```python
from mycelix import LivingProtocolClient
from mycelix.client import ConnectionError, RpcError

async def main():
    try:
        async with LivingProtocolClient("ws://localhost:8888") as client:
            state = await client.get_current_state()
    except ConnectionError as e:
        print(f"Failed to connect: {e}")
    except RpcError as e:
        print(f"RPC failed: {e.code} - {e.message}")
```

## Development

Run tests:

```bash
cd sdk/python
pip install -e ".[dev]"
pytest
```

Run type checking:

```bash
mypy src/mycelix
```

Run linting:

```bash
ruff check src/mycelix
```

## License

MIT License
