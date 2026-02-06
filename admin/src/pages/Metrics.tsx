import { useState } from 'react';
import { Card, SimpleLineChart, SimpleBarChart, PhaseBadge } from '../components';
import { usePolling, useApi } from '../hooks/useApi';
import { apiClient, CyclePhase, PhaseMetrics } from '../api/client';

const PHASE_ORDER: CyclePhase[] = [
  CyclePhase.Shadow,
  CyclePhase.Composting,
  CyclePhase.Liminal,
  CyclePhase.NegativeCapability,
  CyclePhase.Eros,
  CyclePhase.CoCreation,
  CyclePhase.Beauty,
  CyclePhase.EmergentPersonhood,
  CyclePhase.Kenosis,
];

// Mock historical data - in production this would come from the API
const mockHistoricalData = [
  { name: 'Cycle 1', spectralK: 0.42, trust: 65, agents: 45 },
  { name: 'Cycle 2', spectralK: 0.48, trust: 68, agents: 52 },
  { name: 'Cycle 3', spectralK: 0.45, trust: 72, agents: 61 },
  { name: 'Cycle 4', spectralK: 0.51, trust: 70, agents: 58 },
  { name: 'Cycle 5', spectralK: 0.55, trust: 75, agents: 72 },
];

export default function Metrics() {
  const [selectedPhase, setSelectedPhase] = useState<CyclePhase | null>(null);

  const { data: cycleState } = usePolling(() => apiClient.getCycleState(), 5000);

  const { data: currentMetrics, loading: loadingCurrent } = usePolling(
    () => apiClient.getPhaseMetrics(),
    5000
  );

  const { data: phaseMetrics, loading: loadingPhase } = useApi(
    () =>
      selectedPhase
        ? apiClient.getPhaseMetrics(selectedPhase)
        : Promise.resolve(null as PhaseMetrics | null),
    [selectedPhase]
  );

  const displayMetrics = selectedPhase ? phaseMetrics : currentMetrics;
  const loading = selectedPhase ? loadingPhase : loadingCurrent;

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Metrics</h1>
        <p className="text-gray-400 mt-1">Detailed protocol metrics and analytics</p>
      </div>

      {/* Phase Selector */}
      <Card title="Phase Selection" className="mb-6">
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setSelectedPhase(null)}
            className={`px-3 py-2 rounded-lg text-sm transition-colors ${
              selectedPhase === null
                ? 'bg-mycelix-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            Current Phase
            {cycleState && (
              <span className="ml-2 text-xs opacity-70">
                ({cycleState.currentPhase})
              </span>
            )}
          </button>
          {PHASE_ORDER.map((phase) => (
            <button
              key={phase}
              onClick={() => setSelectedPhase(phase)}
              className={`px-3 py-2 rounded-lg text-sm transition-colors ${
                selectedPhase === phase
                  ? 'bg-mycelix-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {phase}
            </button>
          ))}
        </div>
      </Card>

      {/* Current/Selected Phase Metrics */}
      <Card
        title={
          <div className="flex items-center gap-3">
            <span>Phase Metrics</span>
            {selectedPhase ? (
              <PhaseBadge phase={selectedPhase} />
            ) : cycleState ? (
              <PhaseBadge phase={cycleState.currentPhase} />
            ) : null}
          </div>
        }
        className="mb-6"
      >
        {loading ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-mycelix-500"></div>
          </div>
        ) : displayMetrics ? (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
            <MetricBox
              label="Active Agents"
              value={displayMetrics.activeAgents}
              icon={
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              }
            />
            <MetricBox
              label="Spectral K"
              value={displayMetrics.spectralK.toFixed(4)}
              icon={
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
                </svg>
              }
            />
            <MetricBox
              label="Mean Trust"
              value={`${(displayMetrics.meanMetabolicTrust * 100).toFixed(1)}%`}
              icon={
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
              }
            />
            <MetricBox
              label="Active Wounds"
              value={displayMetrics.activeWounds}
              icon={
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
                </svg>
              }
            />
            <MetricBox
              label="Composting Entities"
              value={displayMetrics.compostingEntities}
            />
            <MetricBox
              label="Liminal Entities"
              value={displayMetrics.liminalEntities}
            />
            <MetricBox
              label="Entangled Pairs"
              value={displayMetrics.entangledPairs}
            />
            <MetricBox
              label="Held Uncertainties"
              value={displayMetrics.heldUncertainties}
            />
          </div>
        ) : (
          <div className="text-center text-gray-400 py-8">
            No metrics available
          </div>
        )}
      </Card>

      {/* Historical Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Spectral K Over Cycles">
          <SimpleLineChart
            data={mockHistoricalData}
            lines={[{ key: 'spectralK', color: '#0ea5e9', name: 'Spectral K' }]}
            height={250}
          />
        </Card>

        <Card title="Trust & Agent Growth">
          <SimpleBarChart
            data={mockHistoricalData}
            bars={[
              { key: 'trust', color: '#10b981', name: 'Mean Trust (%)' },
              { key: 'agents', color: '#6366f1', name: 'Active Agents' },
            ]}
            height={250}
          />
        </Card>
      </div>
    </div>
  );
}

interface MetricBoxProps {
  label: string;
  value: string | number;
  icon?: React.ReactNode;
}

function MetricBox({ label, value, icon }: MetricBoxProps) {
  return (
    <div className="bg-gray-700/50 rounded-lg p-4">
      <div className="flex items-center gap-2 text-gray-400 text-sm mb-2">
        {icon && <span className="text-mycelix-400">{icon}</span>}
        {label}
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
    </div>
  );
}
