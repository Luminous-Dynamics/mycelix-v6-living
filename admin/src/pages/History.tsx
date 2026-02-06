import { Card, Table, PhaseBadge } from '../components';
import { usePolling } from '../hooks/useApi';
import { apiClient, PhaseTransition } from '../api/client';

export default function History() {
  const { data: history, loading } = usePolling(
    () => apiClient.getTransitionHistory(),
    10000
  );

  const columns = [
    {
      key: 'cycleNumber',
      header: 'Cycle',
      className: 'w-20',
      render: (transition: PhaseTransition) => (
        <span className="font-mono text-mycelix-400">#{transition.cycleNumber}</span>
      ),
    },
    {
      key: 'from',
      header: 'From',
      render: (transition: PhaseTransition) => (
        <PhaseBadge phase={transition.from} />
      ),
    },
    {
      key: 'arrow',
      header: '',
      className: 'w-12',
      render: () => (
        <svg
          className="w-5 h-5 text-gray-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M13 7l5 5m0 0l-5 5m5-5H6"
          />
        </svg>
      ),
    },
    {
      key: 'to',
      header: 'To',
      render: (transition: PhaseTransition) => (
        <PhaseBadge phase={transition.to} />
      ),
    },
    {
      key: 'transitionedAt',
      header: 'Timestamp',
      render: (transition: PhaseTransition) => {
        const date = new Date(transition.transitionedAt);
        return (
          <div>
            <div className="text-white">{date.toLocaleDateString()}</div>
            <div className="text-xs text-gray-500">{date.toLocaleTimeString()}</div>
          </div>
        );
      },
    },
  ];

  // Group by cycle for summary
  const cycleGroups = history?.reduce((acc, transition) => {
    const cycle = transition.cycleNumber;
    if (!acc[cycle]) {
      acc[cycle] = [];
    }
    acc[cycle].push(transition);
    return acc;
  }, {} as Record<number, PhaseTransition[]>) ?? {};

  const cycleCount = Object.keys(cycleGroups).length;
  const totalTransitions = history?.length ?? 0;

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Phase Transition History</h1>
        <p className="text-gray-400 mt-1">Complete log of cycle phase transitions</p>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-white">{cycleCount}</div>
          <div className="text-sm text-gray-400">Cycles Completed</div>
        </div>
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-white">{totalTransitions}</div>
          <div className="text-sm text-gray-400">Total Transitions</div>
        </div>
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-white">
            {history && history.length > 0
              ? new Date(history[history.length - 1].transitionedAt).toLocaleDateString()
              : '-'}
          </div>
          <div className="text-sm text-gray-400">Last Transition</div>
        </div>
      </div>

      {/* Transition Table */}
      <Card title="Transition Log">
        <Table
          columns={columns}
          data={history ?? []}
          keyExtractor={(t, index) => `${t.cycleNumber}-${t.from}-${t.to}-${index}`}
          loading={loading}
          emptyMessage="No transitions recorded yet"
        />
      </Card>

      {/* Cycle Timeline (condensed view) */}
      {cycleCount > 0 && (
        <Card title="Cycle Timeline" className="mt-6">
          <div className="space-y-4">
            {Object.entries(cycleGroups)
              .reverse()
              .slice(0, 5)
              .map(([cycle, transitions]) => (
                <div key={cycle} className="border-b border-gray-700 pb-4 last:border-0">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-lg font-bold text-mycelix-400">Cycle #{cycle}</span>
                    <span className="text-sm text-gray-500">
                      ({transitions.length} transitions)
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-1">
                    {transitions.map((t, idx) => (
                      <div key={idx} className="flex items-center gap-1">
                        {idx === 0 && <PhaseBadge phase={t.from} className="text-xs" />}
                        <svg
                          className="w-3 h-3 text-gray-500"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M9 5l7 7-7 7"
                          />
                        </svg>
                        <PhaseBadge phase={t.to} className="text-xs" />
                      </div>
                    ))}
                  </div>
                </div>
              ))}
          </div>
        </Card>
      )}
    </div>
  );
}
