import { Card, StatCard, PhaseBadge, SimpleLineChart } from '../components';
import { usePolling } from '../hooks/useApi';
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

// Mock data for chart - in production this would come from the API
const mockConnectionData = [
  { name: '10:00', connections: 45, messages: 120 },
  { name: '10:05', connections: 52, messages: 156 },
  { name: '10:10', connections: 48, messages: 142 },
  { name: '10:15', connections: 61, messages: 189 },
  { name: '10:20', connections: 58, messages: 175 },
  { name: '10:25', connections: 65, messages: 198 },
  { name: '10:30', connections: 72, messages: 225 },
];

export default function Dashboard() {
  const { data: cycleState, loading: loadingCycle } = usePolling(
    () => apiClient.getCycleState(),
    5000
  );

  const { data: serverMetrics, loading: loadingMetrics } = usePolling(
    () => apiClient.getServerMetrics(),
    5000
  );

  const { data: phaseMetrics } = usePolling(
    () => apiClient.getPhaseMetrics(),
    10000
  );

  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  };

  const currentPhaseIndex = cycleState
    ? PHASE_ORDER.indexOf(cycleState.currentPhase)
    : 0;

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Dashboard</h1>
        <p className="text-gray-400 mt-1">Living Protocol Server Overview</p>
      </div>

      {/* Cycle Status */}
      <Card title="Current Cycle Status" className="mb-6">
        {loadingCycle ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-mycelix-500"></div>
          </div>
        ) : cycleState ? (
          <div className="space-y-4">
            <div className="flex items-center gap-4">
              <div>
                <span className="text-gray-400 text-sm">Current Phase:</span>
                <div className="mt-1">
                  <PhaseBadge phase={cycleState.currentPhase} className="text-lg px-3 py-1" />
                </div>
              </div>
              <div className="ml-auto text-right">
                <span className="text-gray-400 text-sm">Cycle Number</span>
                <div className="text-3xl font-bold text-white">
                  #{cycleState.cycleNumber}
                </div>
              </div>
            </div>

            {/* Phase Progress Bar */}
            <div>
              <div className="flex justify-between text-xs text-gray-500 mb-2">
                {PHASE_ORDER.map((phase, index) => (
                  <span
                    key={phase}
                    className={index <= currentPhaseIndex ? 'text-mycelix-400' : ''}
                  >
                    {phase.slice(0, 3)}
                  </span>
                ))}
              </div>
              <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-mycelix-500 to-mycelix-400 transition-all duration-500"
                  style={{
                    width: `${((currentPhaseIndex + 1) / PHASE_ORDER.length) * 100}%`,
                  }}
                />
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-gray-400">Phase Started:</span>
                <span className="text-white ml-2">
                  {new Date(cycleState.phaseStarted).toLocaleString()}
                </span>
              </div>
              <div>
                <span className="text-gray-400">Day in Phase:</span>
                <span className="text-white ml-2">{cycleState.phaseDay}</span>
              </div>
            </div>
          </div>
        ) : (
          <div className="text-center text-gray-400 py-4">
            Unable to load cycle state
          </div>
        )}
      </Card>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <StatCard
          title="Active Connections"
          value={serverMetrics?.activeConnections ?? '-'}
          subtitle={`${serverMetrics?.totalConnections ?? 0} total`}
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          }
        />
        <StatCard
          title="Messages/sec"
          value={
            serverMetrics && serverMetrics.uptimeSeconds > 0
              ? Math.round(
                  (serverMetrics.messagesReceived + serverMetrics.messagesSent) /
                    serverMetrics.uptimeSeconds
                )
              : '-'
          }
          subtitle="avg throughput"
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
          }
        />
        <StatCard
          title="Uptime"
          value={serverMetrics ? formatUptime(serverMetrics.uptimeSeconds) : '-'}
          subtitle="since start"
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          }
        />
        <StatCard
          title="Active Agents"
          value={phaseMetrics?.activeAgents ?? '-'}
          subtitle="in network"
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          }
        />
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card title="Connection Activity">
          <SimpleLineChart
            data={mockConnectionData}
            lines={[
              { key: 'connections', color: '#0ea5e9', name: 'Connections' },
            ]}
            height={250}
          />
        </Card>

        <Card title="Message Throughput">
          <SimpleLineChart
            data={mockConnectionData}
            lines={[
              { key: 'messages', color: '#10b981', name: 'Messages' },
            ]}
            height={250}
          />
        </Card>
      </div>

      {/* Phase Metrics */}
      {phaseMetrics && (
        <Card title="Current Phase Metrics" className="mt-6">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="text-center">
              <div className="text-2xl font-bold text-white">{phaseMetrics.spectralK.toFixed(3)}</div>
              <div className="text-xs text-gray-400">Spectral K</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">{(phaseMetrics.meanMetabolicTrust * 100).toFixed(1)}%</div>
              <div className="text-xs text-gray-400">Mean Trust</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">{phaseMetrics.entangledPairs}</div>
              <div className="text-xs text-gray-400">Entangled Pairs</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">{phaseMetrics.activeWounds}</div>
              <div className="text-xs text-gray-400">Active Wounds</div>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}
