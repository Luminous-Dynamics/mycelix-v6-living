import React from 'react';
import { CycleState } from '@mycelix/living-protocol-sdk';
import { getPhaseInfo } from '../hooks/useLivingProtocol';

interface CycleStatusProps {
  cycleState: CycleState | null;
  isConnected: boolean;
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    background: 'rgba(255, 255, 255, 0.05)',
    borderRadius: '12px',
    padding: '24px',
    marginBottom: '24px',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '20px',
  },
  title: {
    fontSize: '18px',
    fontWeight: 600,
    color: '#e4e4e7',
  },
  connectionBadge: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    fontSize: '12px',
    padding: '4px 12px',
    borderRadius: '16px',
    background: 'rgba(0, 0, 0, 0.2)',
  },
  connectionDot: {
    width: '8px',
    height: '8px',
    borderRadius: '50%',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    gap: '20px',
  },
  statCard: {
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '8px',
    padding: '16px',
    textAlign: 'center' as const,
  },
  statLabel: {
    fontSize: '12px',
    color: '#a1a1aa',
    marginBottom: '8px',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.5px',
  },
  statValue: {
    fontSize: '28px',
    fontWeight: 700,
  },
  phaseCard: {
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '8px',
    padding: '16px',
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
  },
  phaseName: {
    fontSize: '24px',
    fontWeight: 700,
    marginBottom: '4px',
  },
  phaseDescription: {
    fontSize: '12px',
    color: '#a1a1aa',
    textAlign: 'center' as const,
  },
  loading: {
    textAlign: 'center' as const,
    padding: '40px',
    color: '#a1a1aa',
  },
};

export function CycleStatus({ cycleState, isConnected }: CycleStatusProps): JSX.Element {
  if (!cycleState) {
    return (
      <div style={styles.container}>
        <div style={styles.loading}>
          {isConnected ? 'Loading cycle state...' : 'Connecting to server...'}
        </div>
      </div>
    );
  }

  const phaseInfo = getPhaseInfo(cycleState.currentPhase);
  const phaseDaysRemaining = phaseInfo.duration - cycleState.phaseDay;

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h2 style={styles.title}>Cycle Status</h2>
        <div style={styles.connectionBadge}>
          <div
            style={{
              ...styles.connectionDot,
              background: isConnected ? '#4ade80' : '#ef4444',
            }}
          />
          <span>{isConnected ? 'Connected' : 'Disconnected'}</span>
        </div>
      </div>

      <div style={styles.grid}>
        <div style={styles.statCard}>
          <div style={styles.statLabel}>Cycle Number</div>
          <div style={styles.statValue}>{cycleState.cycleNumber}</div>
        </div>

        <div
          style={{
            ...styles.phaseCard,
            borderLeft: `4px solid ${phaseInfo.color}`,
          }}
        >
          <div style={{ ...styles.phaseName, color: phaseInfo.color }}>
            {phaseInfo.name}
          </div>
          <div style={styles.phaseDescription}>{phaseInfo.description}</div>
        </div>

        <div style={styles.statCard}>
          <div style={styles.statLabel}>Phase Day</div>
          <div style={styles.statValue}>
            {cycleState.phaseDay}
            <span style={{ fontSize: '16px', color: '#a1a1aa' }}>
              {' / '}{phaseInfo.duration}
            </span>
          </div>
        </div>
      </div>

      <div style={{ marginTop: '16px', textAlign: 'center' as const }}>
        <span style={{ fontSize: '14px', color: '#a1a1aa' }}>
          {phaseDaysRemaining > 0
            ? `${phaseDaysRemaining} day${phaseDaysRemaining !== 1 ? 's' : ''} remaining in phase`
            : 'Phase transition imminent'}
        </span>
      </div>
    </div>
  );
}
