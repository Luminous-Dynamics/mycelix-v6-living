import React from 'react';
import { PhaseTransition } from '@mycelix/living-protocol-sdk';
import { getPhaseInfo } from '../hooks/useLivingProtocol';

interface TransitionHistoryProps {
  transitions: PhaseTransition[];
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
    marginBottom: '16px',
  },
  title: {
    fontSize: '18px',
    fontWeight: 600,
    color: '#e4e4e7',
  },
  count: {
    fontSize: '12px',
    color: '#a1a1aa',
    background: 'rgba(0, 0, 0, 0.2)',
    padding: '4px 12px',
    borderRadius: '12px',
  },
  list: {
    maxHeight: '300px',
    overflowY: 'auto' as const,
  },
  item: {
    display: 'flex',
    alignItems: 'center',
    padding: '12px',
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '8px',
    marginBottom: '8px',
  },
  arrow: {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    flex: 1,
  },
  phaseTag: {
    padding: '4px 10px',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 500,
  },
  arrowIcon: {
    color: '#71717a',
    fontSize: '16px',
  },
  meta: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'flex-end',
    gap: '2px',
  },
  cycle: {
    fontSize: '12px',
    color: '#a1a1aa',
  },
  time: {
    fontSize: '11px',
    color: '#71717a',
  },
  empty: {
    textAlign: 'center' as const,
    padding: '40px',
    color: '#71717a',
  },
  metricsPreview: {
    display: 'flex',
    gap: '12px',
    marginTop: '8px',
    paddingTop: '8px',
    borderTop: '1px solid rgba(255, 255, 255, 0.05)',
    fontSize: '11px',
    color: '#71717a',
  },
  metric: {
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
  },
  metricValue: {
    color: '#a1a1aa',
    fontWeight: 500,
  },
};

function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    return `Today at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  } else if (diffDays === 1) {
    return 'Yesterday';
  } else if (diffDays < 7) {
    return `${diffDays} days ago`;
  } else {
    return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }
}

export function TransitionHistory({ transitions }: TransitionHistoryProps): JSX.Element {
  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h2 style={styles.title}>Phase Transitions</h2>
        <span style={styles.count}>{transitions.length} transitions</span>
      </div>

      {transitions.length === 0 ? (
        <div style={styles.empty}>No transitions recorded yet</div>
      ) : (
        <div style={styles.list}>
          {transitions.map((transition, index) => {
            const fromInfo = getPhaseInfo(transition.from);
            const toInfo = getPhaseInfo(transition.to);

            return (
              <div key={index} style={styles.item}>
                <div style={styles.arrow}>
                  <span
                    style={{
                      ...styles.phaseTag,
                      background: `${fromInfo.color}20`,
                      color: fromInfo.color,
                    }}
                  >
                    {fromInfo.name}
                  </span>
                  <span style={styles.arrowIcon}>-&gt;</span>
                  <span
                    style={{
                      ...styles.phaseTag,
                      background: `${toInfo.color}20`,
                      color: toInfo.color,
                    }}
                  >
                    {toInfo.name}
                  </span>
                </div>
                <div style={styles.meta}>
                  <span style={styles.cycle}>Cycle {transition.cycleNumber}</span>
                  <span style={styles.time}>{formatTimestamp(transition.transitionedAt)}</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/**
 * Detailed view of a single transition with metrics.
 */
export function TransitionDetail({
  transition,
}: {
  transition: PhaseTransition;
}): JSX.Element {
  const fromInfo = getPhaseInfo(transition.from);
  const toInfo = getPhaseInfo(transition.to);
  const { metrics } = transition;

  return (
    <div style={styles.item}>
      <div style={{ flex: 1 }}>
        <div style={styles.arrow}>
          <span
            style={{
              ...styles.phaseTag,
              background: `${fromInfo.color}20`,
              color: fromInfo.color,
            }}
          >
            {fromInfo.name}
          </span>
          <span style={styles.arrowIcon}>-&gt;</span>
          <span
            style={{
              ...styles.phaseTag,
              background: `${toInfo.color}20`,
              color: toInfo.color,
            }}
          >
            {toInfo.name}
          </span>
        </div>
        <div style={styles.metricsPreview}>
          <div style={styles.metric}>
            Agents: <span style={styles.metricValue}>{metrics.activeAgents}</span>
          </div>
          <div style={styles.metric}>
            Spectral K: <span style={styles.metricValue}>{metrics.spectralK.toFixed(2)}</span>
          </div>
          <div style={styles.metric}>
            Wounds: <span style={styles.metricValue}>{metrics.activeWounds}</span>
          </div>
          <div style={styles.metric}>
            Pairs: <span style={styles.metricValue}>{metrics.entangledPairs}</span>
          </div>
        </div>
      </div>
      <div style={styles.meta}>
        <span style={styles.cycle}>Cycle {transition.cycleNumber}</span>
        <span style={styles.time}>{formatTimestamp(transition.transitionedAt)}</span>
      </div>
    </div>
  );
}
