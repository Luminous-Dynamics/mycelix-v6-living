import { useState } from 'react';
import { Card, Button, PhaseBadge } from '../components';
import { usePolling, useApi } from '../hooks/useApi';
import { apiClient, CyclePhase } from '../api/client';

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

const PHASE_DESCRIPTIONS: Record<CyclePhase, string> = {
  [CyclePhase.Shadow]: 'Surfacing hidden aspects and suppressed knowledge. Gate 2 warnings suspended.',
  [CyclePhase.Composting]: 'Decomposing failed proposals, extracting learnings as nutrients.',
  [CyclePhase.Liminal]: 'Threshold state - entities in transition between states.',
  [CyclePhase.NegativeCapability]: 'Holding uncertainty without reaching for resolution. Voting blocked.',
  [CyclePhase.Eros]: 'Computing attractor basins and resonance patterns.',
  [CyclePhase.CoCreation]: 'Standard consensus and collaborative decision-making.',
  [CyclePhase.Beauty]: 'Validating proposals through aesthetic/harmonic criteria.',
  [CyclePhase.EmergentPersonhood]: 'Measuring collective consciousness emergence.',
  [CyclePhase.Kenosis]: 'Voluntary reputation release - agents empty themselves for renewal.',
};

const PHASE_DURATIONS: Record<CyclePhase, number> = {
  [CyclePhase.Shadow]: 2,
  [CyclePhase.Composting]: 5,
  [CyclePhase.Liminal]: 3,
  [CyclePhase.NegativeCapability]: 3,
  [CyclePhase.Eros]: 4,
  [CyclePhase.CoCreation]: 7,
  [CyclePhase.Beauty]: 2,
  [CyclePhase.EmergentPersonhood]: 1,
  [CyclePhase.Kenosis]: 1,
};

export default function CycleControl() {
  const [advancing, setAdvancing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { data: cycleState, refetch } = usePolling(
    () => apiClient.getCycleState(),
    2000
  );

  const { data: config } = useApi(() => apiClient.getConfig(), []);

  const handleAdvance = async () => {
    if (!config?.testMode) {
      setError('Cycle advancement is only available in test mode');
      return;
    }

    setAdvancing(true);
    setError(null);
    try {
      await apiClient.advanceCycle();
      refetch();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to advance cycle');
    } finally {
      setAdvancing(false);
    }
  };

  const currentPhaseIndex = cycleState
    ? PHASE_ORDER.indexOf(cycleState.currentPhase)
    : -1;

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Cycle Control</h1>
        <p className="text-gray-400 mt-1">View and control the metabolism cycle state</p>
      </div>

      {/* Test Mode Warning */}
      {config && !config.testMode && (
        <div className="mb-6 p-4 bg-yellow-900/30 border border-yellow-700 rounded-lg">
          <div className="flex items-center gap-2 text-yellow-400">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            <span className="font-medium">Production Mode</span>
          </div>
          <p className="text-yellow-300/70 text-sm mt-1">
            Manual cycle advancement is disabled. Start the server with <code className="bg-yellow-800/50 px-1 rounded">--simulated-time</code> to enable testing controls.
          </p>
        </div>
      )}

      {error && (
        <div className="mb-6 p-4 bg-red-900/30 border border-red-700 rounded-lg text-red-400">
          {error}
        </div>
      )}

      {/* Current State */}
      <Card className="mb-6">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h3 className="text-lg font-medium text-white">Current State</h3>
            <p className="text-gray-400 text-sm">Cycle #{cycleState?.cycleNumber ?? '-'}</p>
          </div>
          {config?.testMode && (
            <Button onClick={handleAdvance} loading={advancing} variant="primary">
              Advance to Next Phase
            </Button>
          )}
        </div>

        {cycleState && (
          <>
            <div className="flex items-center gap-4 mb-6">
              <PhaseBadge phase={cycleState.currentPhase} className="text-lg px-4 py-2" />
              <div className="text-gray-400">
                Day {cycleState.phaseDay + 1} of {PHASE_DURATIONS[cycleState.currentPhase]}
              </div>
            </div>
            <p className="text-gray-300 mb-4">
              {PHASE_DESCRIPTIONS[cycleState.currentPhase]}
            </p>
            <div className="grid grid-cols-2 gap-4 text-sm text-gray-400">
              <div>
                <span>Phase started:</span>
                <span className="text-white ml-2">
                  {new Date(cycleState.phaseStarted).toLocaleString()}
                </span>
              </div>
              <div>
                <span>Cycle started:</span>
                <span className="text-white ml-2">
                  {new Date(cycleState.cycleStarted).toLocaleString()}
                </span>
              </div>
            </div>
          </>
        )}
      </Card>

      {/* Phase Timeline */}
      <Card title="28-Day Lunar Cycle">
        <div className="space-y-3">
          {PHASE_ORDER.map((phase, index) => {
            const isCurrent = cycleState?.currentPhase === phase;
            const isPast = index < currentPhaseIndex;
            const isFuture = index > currentPhaseIndex;

            return (
              <div
                key={phase}
                className={`flex items-center gap-4 p-3 rounded-lg transition-colors ${
                  isCurrent ? 'bg-mycelix-900/50 border border-mycelix-700' : ''
                }`}
              >
                {/* Status Indicator */}
                <div
                  className={`w-3 h-3 rounded-full ${
                    isCurrent
                      ? 'bg-mycelix-500 animate-pulse'
                      : isPast
                      ? 'bg-green-500'
                      : 'bg-gray-600'
                  }`}
                />

                {/* Phase Info */}
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span
                      className={`font-medium ${
                        isCurrent
                          ? 'text-white'
                          : isPast
                          ? 'text-gray-400'
                          : 'text-gray-500'
                      }`}
                    >
                      {phase}
                    </span>
                    <span className="text-xs text-gray-500">
                      ({PHASE_DURATIONS[phase]} days)
                    </span>
                    {isCurrent && (
                      <span className="text-xs bg-mycelix-600 text-white px-2 py-0.5 rounded">
                        Current
                      </span>
                    )}
                  </div>
                  <p
                    className={`text-sm mt-1 ${
                      isFuture ? 'text-gray-600' : 'text-gray-400'
                    }`}
                  >
                    {PHASE_DESCRIPTIONS[phase]}
                  </p>
                </div>

                {/* Day Progress (for current phase) */}
                {isCurrent && cycleState && (
                  <div className="text-right">
                    <div className="text-2xl font-bold text-mycelix-400">
                      {cycleState.phaseDay + 1}/{PHASE_DURATIONS[phase]}
                    </div>
                    <div className="text-xs text-gray-500">days</div>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="mt-6 pt-4 border-t border-gray-700 flex justify-between text-sm text-gray-400">
          <span>Total cycle: 28 days</span>
          <span>
            Progress:{' '}
            {cycleState
              ? Math.round(((currentPhaseIndex + 1) / PHASE_ORDER.length) * 100)
              : 0}
            %
          </span>
        </div>
      </Card>
    </div>
  );
}
