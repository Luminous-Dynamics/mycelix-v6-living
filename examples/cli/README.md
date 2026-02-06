# Living Protocol CLI

A command-line interface for monitoring and interacting with the Living Protocol.

## Features

- **status** - Display current cycle state with optional timeline
- **watch** - Stream live events with optional filtering
- **history** - View phase transition history
- **metrics** - Display network metrics
- **phases** - List all phases with descriptions

## Prerequisites

- Node.js 18+
- A running Living Protocol server with WebSocket support

## Installation

1. Install dependencies:

```bash
npm install
```

2. Build the CLI:

```bash
npm run build
```

3. (Optional) Link globally:

```bash
npm link
```

## Usage

### Basic Usage

```bash
# With ts-node (development)
npm run dev -- status

# With compiled JS
npm start -- status

# If globally linked
living-protocol status
```

### Commands

#### Status

Show the current cycle status:

```bash
living-protocol status
```

With timeline:

```bash
living-protocol status --timeline
```

Example output:

```
Cycle Status
────────────────────────────────────────
  Cycle Number:  3
  Current Phase: CoCreation
  Description:   Collaborative emergence
  Phase Day:     4 / 7
  Phase Started: 2/2/2026, 10:00:00 AM
  Cycle Started: 1/15/2026, 12:00:00 AM
  Progress:      [████████████░░░░░░░░░░░░░░░░░░] 57%
```

#### Watch

Stream live events:

```bash
living-protocol watch
```

Filter by event types:

```bash
living-protocol watch --filter PhaseTransitioned,WoundCreated
```

Example output:

```
Watching for events...
Press Ctrl+C to stop
────────────────────────────────────────────────────────────

10:15:23 PhaseTransitioned {"from":"Eros","to":"CoCreation",...}
10:15:45 WoundCreated {"woundId":"w123","agentDid":"did:...","severity":"Minor"}
10:16:02 EntanglementFormed {"agentA":"did:...","agentB":"did:...","strength":0.85}
```

#### History

View phase transition history:

```bash
living-protocol history
```

Limit results:

```bash
living-protocol history --limit 5
```

#### Metrics

Show network metrics:

```bash
living-protocol metrics
```

Get metrics for a specific phase:

```bash
living-protocol metrics --phase Composting
```

Example output:

```
Network Metrics
────────────────────────────────────────
  Active Agents:       42
  Spectral K:          0.782
  Metabolic Trust:     68.5%
  Active Wounds:       3
  Composting Entities: 7
  Liminal Entities:    2
  Entangled Pairs:     15
  Held Uncertainties:  4
```

#### Phases

List all cycle phases with details:

```bash
living-protocol phases
```

### Global Options

```bash
# Specify custom WebSocket URL
living-protocol --url ws://custom-server:8888/ws status

# Show help
living-protocol --help

# Show version
living-protocol --version
```

## Configuration

### WebSocket URL

The default WebSocket URL is `ws://localhost:8888/ws`. Override it with the `--url` flag:

```bash
living-protocol --url ws://production-server:8888/ws status
```

Or set the `LIVING_PROTOCOL_URL` environment variable (requires code modification).

## The 9 Phases

| Phase | Days | Description |
|-------|------|-------------|
| Shadow | 2 | Integration of suppressed content |
| Composting | 5 | Decomposition and nutrient extraction |
| Liminal | 3 | Threshold state between identities |
| Negative Capability | 3 | Holding uncertainty without resolution |
| Eros | 4 | Attraction and creative tension |
| Co-Creation | 7 | Collaborative emergence |
| Beauty | 2 | Aesthetic validation |
| Emergent Personhood | 1 | Network consciousness assessment |
| Kenosis | 1 | Voluntary release and emptying |

**Total: 28 days (lunar cycle)**

## Development

Run in development mode with ts-node:

```bash
npm run dev -- <command>
```

Build for production:

```bash
npm run build
```

## License

AGPL-3.0-or-later
