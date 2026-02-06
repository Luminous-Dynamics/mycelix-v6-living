import React from 'react';
import { CyclePhase, PHASE_ORDER } from '@mycelix/living-protocol-sdk';
import { getPhaseInfo } from '../hooks/useLivingProtocol';

interface PhaseTimelineProps {
  currentPhase: CyclePhase | null;
  phaseDay: number;
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    background: 'rgba(255, 255, 255, 0.05)',
    borderRadius: '12px',
    padding: '24px',
    marginBottom: '24px',
  },
  title: {
    fontSize: '18px',
    fontWeight: 600,
    color: '#e4e4e7',
    marginBottom: '24px',
  },
  timeline: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    position: 'relative' as const,
    padding: '0 10px',
  },
  connector: {
    position: 'absolute' as const,
    top: '16px',
    left: '40px',
    right: '40px',
    height: '2px',
    background: 'rgba(255, 255, 255, 0.1)',
  },
  connectorProgress: {
    height: '100%',
    background: 'linear-gradient(90deg, #4ade80 0%, #22c55e 100%)',
    transition: 'width 0.3s ease',
  },
  phaseItem: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    position: 'relative' as const,
    zIndex: 1,
    flex: '1',
    maxWidth: '80px',
  },
  phaseCircle: {
    width: '32px',
    height: '32px',
    borderRadius: '50%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '12px',
    fontWeight: 600,
    marginBottom: '8px',
    transition: 'all 0.3s ease',
  },
  phaseName: {
    fontSize: '10px',
    textAlign: 'center' as const,
    color: '#a1a1aa',
    maxWidth: '70px',
  },
  phaseDuration: {
    fontSize: '9px',
    color: '#71717a',
    marginTop: '2px',
  },
  currentIndicator: {
    position: 'absolute' as const,
    top: '-8px',
    left: '50%',
    transform: 'translateX(-50%)',
    width: '0',
    height: '0',
    borderLeft: '6px solid transparent',
    borderRight: '6px solid transparent',
    borderTop: '6px solid #4ade80',
  },
  legend: {
    display: 'flex',
    justifyContent: 'center',
    gap: '24px',
    marginTop: '24px',
    flexWrap: 'wrap' as const,
  },
  legendItem: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    fontSize: '12px',
    color: '#a1a1aa',
  },
  legendDot: {
    width: '12px',
    height: '12px',
    borderRadius: '50%',
  },
};

export function PhaseTimeline({ currentPhase, phaseDay }: PhaseTimelineProps): JSX.Element {
  const currentIndex = currentPhase ? PHASE_ORDER.indexOf(currentPhase) : -1;
  const totalPhases = PHASE_ORDER.length;

  // Calculate progress percentage for the connector line
  const progressPercent = currentPhase
    ? ((currentIndex + phaseDay / getPhaseInfo(currentPhase).duration) / totalPhases) * 100
    : 0;

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>28-Day Lunar Cycle Timeline</h2>

      <div style={styles.timeline}>
        <div style={styles.connector}>
          <div
            style={{
              ...styles.connectorProgress,
              width: `${progressPercent}%`,
            }}
          />
        </div>

        {PHASE_ORDER.map((phase, index) => {
          const info = getPhaseInfo(phase);
          const isPast = index < currentIndex;
          const isCurrent = phase === currentPhase;
          const isFuture = index > currentIndex;

          return (
            <div key={phase} style={styles.phaseItem}>
              {isCurrent && <div style={styles.currentIndicator} />}
              <div
                style={{
                  ...styles.phaseCircle,
                  background: isCurrent
                    ? info.color
                    : isPast
                    ? 'rgba(74, 222, 128, 0.3)'
                    : 'rgba(255, 255, 255, 0.1)',
                  border: isCurrent
                    ? `2px solid ${info.color}`
                    : isPast
                    ? '2px solid #4ade80'
                    : '2px solid rgba(255, 255, 255, 0.1)',
                  color: isCurrent || isPast ? '#fff' : '#71717a',
                  boxShadow: isCurrent ? `0 0 12px ${info.color}80` : 'none',
                }}
              >
                {index + 1}
              </div>
              <div
                style={{
                  ...styles.phaseName,
                  color: isCurrent ? info.color : isFuture ? '#71717a' : '#a1a1aa',
                  fontWeight: isCurrent ? 600 : 400,
                }}
              >
                {info.name}
              </div>
              <div style={styles.phaseDuration}>{info.duration}d</div>
            </div>
          );
        })}
      </div>

      <div style={styles.legend}>
        <div style={styles.legendItem}>
          <div style={{ ...styles.legendDot, background: '#4ade80' }} />
          <span>Completed</span>
        </div>
        <div style={styles.legendItem}>
          <div
            style={{
              ...styles.legendDot,
              background: currentPhase ? getPhaseInfo(currentPhase).color : '#fff',
              boxShadow: `0 0 8px ${currentPhase ? getPhaseInfo(currentPhase).color : '#fff'}80`,
            }}
          />
          <span>Current</span>
        </div>
        <div style={styles.legendItem}>
          <div style={{ ...styles.legendDot, background: 'rgba(255, 255, 255, 0.1)' }} />
          <span>Upcoming</span>
        </div>
      </div>
    </div>
  );
}
