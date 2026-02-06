import React from 'react';
import { useLivingProtocol } from './hooks/useLivingProtocol';
import { CycleStatus } from './components/CycleStatus';
import { PhaseTimeline } from './components/PhaseTimeline';
import { TransitionHistory } from './components/TransitionHistory';
import { MetricsPanel } from './components/MetricsPanel';

const styles: Record<string, React.CSSProperties> = {
  app: {
    minHeight: '100vh',
    padding: '24px',
  },
  header: {
    textAlign: 'center' as const,
    marginBottom: '32px',
  },
  title: {
    fontSize: '32px',
    fontWeight: 700,
    color: '#e4e4e7',
    marginBottom: '8px',
  },
  subtitle: {
    fontSize: '14px',
    color: '#71717a',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '24px',
    maxWidth: '1400px',
    margin: '0 auto',
  },
  fullWidth: {
    gridColumn: '1 / -1',
  },
  errorBanner: {
    background: 'rgba(239, 68, 68, 0.1)',
    border: '1px solid rgba(239, 68, 68, 0.3)',
    borderRadius: '8px',
    padding: '16px',
    marginBottom: '24px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    maxWidth: '1400px',
    margin: '0 auto 24px',
  },
  errorText: {
    color: '#ef4444',
    fontSize: '14px',
  },
  retryButton: {
    background: 'rgba(239, 68, 68, 0.2)',
    border: '1px solid rgba(239, 68, 68, 0.4)',
    borderRadius: '4px',
    padding: '8px 16px',
    color: '#ef4444',
    cursor: 'pointer',
    fontSize: '13px',
  },
  eventsPanel: {
    background: 'rgba(255, 255, 255, 0.05)',
    borderRadius: '12px',
    padding: '24px',
  },
  eventsHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '16px',
  },
  eventsTitle: {
    fontSize: '18px',
    fontWeight: 600,
    color: '#e4e4e7',
  },
  clearButton: {
    background: 'rgba(255, 255, 255, 0.1)',
    border: 'none',
    borderRadius: '4px',
    padding: '6px 12px',
    color: '#a1a1aa',
    cursor: 'pointer',
    fontSize: '12px',
  },
  eventsList: {
    maxHeight: '300px',
    overflowY: 'auto' as const,
  },
  eventItem: {
    padding: '8px 12px',
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '4px',
    marginBottom: '4px',
    fontSize: '12px',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  eventType: {
    color: '#60a5fa',
    fontWeight: 500,
  },
  eventData: {
    color: '#71717a',
    fontSize: '11px',
    maxWidth: '60%',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  },
  footer: {
    textAlign: 'center' as const,
    marginTop: '32px',
    paddingTop: '24px',
    borderTop: '1px solid rgba(255, 255, 255, 0.05)',
    fontSize: '12px',
    color: '#52525b',
  },
};

// Get WebSocket URL from environment or use default
const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:8888/ws';

function App(): JSX.Element {
  const { state, actions } = useLivingProtocol({
    url: WS_URL,
    autoConnect: true,
  });

  return (
    <div style={styles.app}>
      <header style={styles.header}>
        <h1 style={styles.title}>Living Protocol Dashboard</h1>
        <p style={styles.subtitle}>
          Real-time monitoring of the 28-day lunar metabolism cycle
        </p>
      </header>

      {state.error && (
        <div style={styles.errorBanner}>
          <span style={styles.errorText}>
            Connection error: {state.error.message}
          </span>
          <button style={styles.retryButton} onClick={() => actions.connect()}>
            Retry Connection
          </button>
        </div>
      )}

      <div style={styles.grid}>
        <div style={styles.fullWidth}>
          <CycleStatus
            cycleState={state.cycleState}
            isConnected={state.isConnected}
          />
        </div>

        <div style={styles.fullWidth}>
          <PhaseTimeline
            currentPhase={state.cycleState?.currentPhase ?? null}
            phaseDay={state.cycleState?.phaseDay ?? 0}
          />
        </div>

        <div>
          <MetricsPanel
            metrics={state.metrics}
            currentPhase={state.cycleState?.currentPhase ?? null}
          />
        </div>

        <div>
          <TransitionHistory transitions={state.transitionHistory} />
        </div>

        <div style={styles.fullWidth}>
          <div style={styles.eventsPanel}>
            <div style={styles.eventsHeader}>
              <h2 style={styles.eventsTitle}>Live Events</h2>
              <button style={styles.clearButton} onClick={actions.clearEvents}>
                Clear
              </button>
            </div>
            <div style={styles.eventsList}>
              {state.recentEvents.length === 0 ? (
                <div style={{ textAlign: 'center', color: '#71717a', padding: '24px' }}>
                  Waiting for events...
                </div>
              ) : (
                state.recentEvents.map((event, index) => (
                  <div key={index} style={styles.eventItem}>
                    <span style={styles.eventType}>{event.type}</span>
                    <span style={styles.eventData}>
                      {JSON.stringify(event.data)}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>

      <footer style={styles.footer}>
        <p>Mycelix v6.0 Living Protocol Layer</p>
        <p style={{ marginTop: '4px' }}>
          Connection: {state.connectionState} | Events: {state.recentEvents.length}
        </p>
      </footer>
    </div>
  );
}

export default App;
