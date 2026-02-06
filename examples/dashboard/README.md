# Living Protocol Dashboard

A React-based real-time dashboard for monitoring the Living Protocol's 28-day lunar metabolism cycle.

## Features

- **Cycle Status**: Shows current cycle number, phase, and phase day
- **Phase Timeline**: Visual representation of all 9 phases with progress indicator
- **Live Metrics**: Real-time display of network metrics including agent count, spectral K, metabolic trust, and more
- **Transition History**: Log of recent phase transitions with timestamps
- **Live Events**: Stream of protocol events as they occur

## Prerequisites

- Node.js 18+
- A running Living Protocol server with WebSocket support

## Setup

1. Install dependencies:

```bash
npm install
```

2. Configure the WebSocket URL (optional):

Create a `.env` file or set the environment variable:

```bash
VITE_WS_URL=ws://localhost:8888/ws
```

3. Start the development server:

```bash
npm run dev
```

4. Open http://localhost:3000 in your browser.

## Building for Production

```bash
npm run build
```

The built files will be in the `dist/` directory.

## Project Structure

```
dashboard/
├── src/
│   ├── App.tsx                    # Main application component
│   ├── main.tsx                   # Application entry point
│   ├── components/
│   │   ├── CycleStatus.tsx        # Current cycle status display
│   │   ├── PhaseTimeline.tsx      # 9-phase timeline visualization
│   │   ├── TransitionHistory.tsx  # Phase transition log
│   │   └── MetricsPanel.tsx       # Live metrics dashboard
│   └── hooks/
│       └── useLivingProtocol.ts   # React hook for SDK integration
├── index.html                      # HTML template
├── vite.config.ts                  # Vite configuration
├── tsconfig.json                   # TypeScript configuration
└── package.json                    # Dependencies and scripts
```

## The 9 Phases

The Living Protocol operates on a 28-day lunar cycle with 9 distinct phases:

1. **Shadow** (2 days) - Integration of suppressed content
2. **Composting** (5 days) - Decomposition and nutrient extraction
3. **Liminal** (3 days) - Threshold state between identities
4. **Negative Capability** (3 days) - Holding uncertainty without resolution
5. **Eros** (4 days) - Attraction and creative tension
6. **Co-Creation** (7 days) - Collaborative emergence
7. **Beauty** (2 days) - Aesthetic validation
8. **Emergent Personhood** (1 day) - Network consciousness assessment
9. **Kenosis** (1 day) - Voluntary release and emptying

## Using the React Hook

The `useLivingProtocol` hook provides a convenient way to connect to the Living Protocol:

```tsx
import { useLivingProtocol } from './hooks/useLivingProtocol';

function MyComponent() {
  const { state, actions } = useLivingProtocol({
    url: 'ws://localhost:8888/ws',
  });

  if (!state.isConnected) {
    return <div>Connecting...</div>;
  }

  return (
    <div>
      <p>Current Phase: {state.cycleState?.currentPhase}</p>
      <p>Cycle Number: {state.cycleState?.cycleNumber}</p>
      <button onClick={actions.refreshState}>Refresh</button>
    </div>
  );
}
```

## License

AGPL-3.0-or-later
