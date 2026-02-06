import React from 'react';
import { PhaseMetrics, CyclePhase } from '@mycelix/living-protocol-sdk';
import { getPhaseInfo } from '../hooks/useLivingProtocol';

interface MetricsPanelProps {
  metrics: PhaseMetrics | null;
  currentPhase: CyclePhase | null;
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
  liveIndicator: {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    fontSize: '12px',
    color: '#4ade80',
  },
  liveDot: {
    width: '6px',
    height: '6px',
    borderRadius: '50%',
    background: '#4ade80',
    animation: 'pulse 2s infinite',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(4, 1fr)',
    gap: '16px',
  },
  metricCard: {
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '8px',
    padding: '16px',
  },
  metricLabel: {
    fontSize: '11px',
    color: '#71717a',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.5px',
    marginBottom: '8px',
  },
  metricValue: {
    fontSize: '24px',
    fontWeight: 700,
    color: '#e4e4e7',
  },
  metricTrend: {
    fontSize: '11px',
    marginTop: '4px',
  },
  trendUp: {
    color: '#4ade80',
  },
  trendDown: {
    color: '#ef4444',
  },
  trendNeutral: {
    color: '#71717a',
  },
  loading: {
    textAlign: 'center' as const,
    padding: '40px',
    color: '#71717a',
  },
  phaseSpecific: {
    marginTop: '24px',
    padding: '16px',
    background: 'rgba(0, 0, 0, 0.2)',
    borderRadius: '8px',
  },
  phaseSpecificTitle: {
    fontSize: '14px',
    fontWeight: 600,
    marginBottom: '12px',
  },
  progressBar: {
    height: '8px',
    background: 'rgba(255, 255, 255, 0.1)',
    borderRadius: '4px',
    overflow: 'hidden',
    marginTop: '8px',
  },
  progressFill: {
    height: '100%',
    borderRadius: '4px',
    transition: 'width 0.5s ease',
  },
};

interface MetricDisplayProps {
  label: string;
  value: number | string;
  format?: 'number' | 'percent' | 'decimal';
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
  color?: string;
}

function MetricDisplay({
  label,
  value,
  format = 'number',
  trend = 'neutral',
  trendValue,
  color,
}: MetricDisplayProps): JSX.Element {
  let displayValue: string;
  if (typeof value === 'string') {
    displayValue = value;
  } else if (format === 'percent') {
    displayValue = `${(value * 100).toFixed(1)}%`;
  } else if (format === 'decimal') {
    displayValue = value.toFixed(2);
  } else {
    displayValue = value.toLocaleString();
  }

  return (
    <div style={styles.metricCard}>
      <div style={styles.metricLabel}>{label}</div>
      <div style={{ ...styles.metricValue, color: color || styles.metricValue.color }}>
        {displayValue}
      </div>
      {trendValue && (
        <div
          style={{
            ...styles.metricTrend,
            ...(trend === 'up'
              ? styles.trendUp
              : trend === 'down'
              ? styles.trendDown
              : styles.trendNeutral),
          }}
        >
          {trend === 'up' ? '+' : trend === 'down' ? '' : ''}{trendValue}
        </div>
      )}
    </div>
  );
}

export function MetricsPanel({ metrics, currentPhase }: MetricsPanelProps): JSX.Element {
  if (!metrics) {
    return (
      <div style={styles.container}>
        <div style={styles.loading}>Loading metrics...</div>
      </div>
    );
  }

  const phaseInfo = currentPhase ? getPhaseInfo(currentPhase) : null;

  // Calculate network health score (simplified)
  const healthScore =
    (metrics.meanMetabolicTrust * 0.3 +
      (1 - metrics.activeWounds / Math.max(metrics.activeAgents, 1)) * 0.2 +
      (metrics.entangledPairs / Math.max(metrics.activeAgents / 2, 1)) * 0.2 +
      metrics.spectralK * 0.3) *
    100;

  return (
    <div style={styles.container}>
      <style>
        {`
          @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
          }
        `}
      </style>

      <div style={styles.header}>
        <h2 style={styles.title}>Live Metrics</h2>
        <div style={styles.liveIndicator}>
          <div style={styles.liveDot} />
          <span>Live</span>
        </div>
      </div>

      <div style={styles.grid}>
        <MetricDisplay
          label="Active Agents"
          value={metrics.activeAgents}
          color="#4ade80"
        />
        <MetricDisplay
          label="Spectral K"
          value={metrics.spectralK}
          format="decimal"
          color="#60a5fa"
        />
        <MetricDisplay
          label="Metabolic Trust"
          value={metrics.meanMetabolicTrust}
          format="percent"
          color="#fbbf24"
        />
        <MetricDisplay
          label="Network Health"
          value={`${Math.round(healthScore)}%`}
          color={healthScore > 70 ? '#4ade80' : healthScore > 40 ? '#fbbf24' : '#ef4444'}
        />
      </div>

      <div style={{ ...styles.grid, marginTop: '16px' }}>
        <MetricDisplay
          label="Active Wounds"
          value={metrics.activeWounds}
          color={metrics.activeWounds > 5 ? '#ef4444' : '#a1a1aa'}
        />
        <MetricDisplay
          label="Composting Entities"
          value={metrics.compostingEntities}
        />
        <MetricDisplay
          label="Entangled Pairs"
          value={metrics.entangledPairs}
          color="#e879f9"
        />
        <MetricDisplay
          label="Held Uncertainties"
          value={metrics.heldUncertainties}
          color="#818cf8"
        />
      </div>

      {currentPhase && phaseInfo && (
        <div style={styles.phaseSpecific}>
          <div style={{ ...styles.phaseSpecificTitle, color: phaseInfo.color }}>
            Phase-Specific Metrics: {phaseInfo.name}
          </div>
          {currentPhase === CyclePhase.Composting && (
            <div>
              <div style={{ fontSize: '13px', color: '#a1a1aa' }}>
                Decomposition Progress
              </div>
              <div style={styles.progressBar}>
                <div
                  style={{
                    ...styles.progressFill,
                    width: `${Math.min(100, metrics.compostingEntities * 10)}%`,
                    background: phaseInfo.color,
                  }}
                />
              </div>
            </div>
          )}
          {currentPhase === CyclePhase.NegativeCapability && (
            <div>
              <div style={{ fontSize: '13px', color: '#a1a1aa' }}>
                Claims held in uncertainty: {metrics.heldUncertainties}
              </div>
              <div style={{ fontSize: '12px', color: '#71717a', marginTop: '4px' }}>
                Voting is currently blocked during this phase
              </div>
            </div>
          )}
          {currentPhase === CyclePhase.Liminal && (
            <div>
              <div style={{ fontSize: '13px', color: '#a1a1aa' }}>
                Entities in liminal transition: {metrics.liminalEntities}
              </div>
            </div>
          )}
          {(currentPhase === CyclePhase.Eros || currentPhase === CyclePhase.CoCreation) && (
            <div>
              <div style={{ fontSize: '13px', color: '#a1a1aa' }}>
                Entanglement Network Density
              </div>
              <div style={styles.progressBar}>
                <div
                  style={{
                    ...styles.progressFill,
                    width: `${Math.min(100, (metrics.entangledPairs / Math.max(metrics.activeAgents, 1)) * 100)}%`,
                    background: phaseInfo.color,
                  }}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
