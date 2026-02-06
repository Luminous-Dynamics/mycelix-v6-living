import { Card, Table, Badge } from '../components';
import { usePolling } from '../hooks/useApi';
import { apiClient, ConnectionInfo } from '../api/client';

export default function Connections() {
  const { data: connections, loading } = usePolling(
    () => apiClient.getConnections(),
    3000
  );

  const { data: serverMetrics } = usePolling(
    () => apiClient.getServerMetrics(),
    3000
  );

  const columns = [
    {
      key: 'id',
      header: 'ID',
      className: 'w-16',
      render: (conn: ConnectionInfo) => (
        <span className="font-mono text-gray-400">#{conn.id}</span>
      ),
    },
    {
      key: 'remoteAddr',
      header: 'Remote Address',
      render: (conn: ConnectionInfo) => (
        <span className="font-mono">{conn.remoteAddr}</span>
      ),
    },
    {
      key: 'connectedAt',
      header: 'Connected',
      render: (conn: ConnectionInfo) => {
        const date = new Date(conn.connectedAt);
        const duration = Math.floor((Date.now() - date.getTime()) / 1000);
        const formatDuration = (s: number) => {
          if (s < 60) return `${s}s`;
          if (s < 3600) return `${Math.floor(s / 60)}m`;
          return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
        };
        return (
          <div>
            <div>{date.toLocaleTimeString()}</div>
            <div className="text-xs text-gray-500">{formatDuration(duration)} ago</div>
          </div>
        );
      },
    },
    {
      key: 'authenticated',
      header: 'Auth',
      className: 'w-24',
      render: (conn: ConnectionInfo) => (
        <Badge variant={conn.authenticated ? 'success' : 'default'}>
          {conn.authenticated ? 'Yes' : 'No'}
        </Badge>
      ),
    },
    {
      key: 'messages',
      header: 'Messages',
      render: (conn: ConnectionInfo) => (
        <div className="text-right">
          <span className="text-green-400">{conn.messagesReceived}</span>
          <span className="text-gray-500 mx-1">/</span>
          <span className="text-blue-400">{conn.messagesSent}</span>
          <div className="text-xs text-gray-500">recv / sent</div>
        </div>
      ),
    },
  ];

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Active Connections</h1>
        <p className="text-gray-400 mt-1">Monitor connected WebSocket clients</p>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-white">
            {serverMetrics?.activeConnections ?? '-'}
          </div>
          <div className="text-sm text-gray-400">Active Connections</div>
        </div>
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-white">
            {serverMetrics?.totalConnections ?? '-'}
          </div>
          <div className="text-sm text-gray-400">Total (All Time)</div>
        </div>
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-green-400">
            {serverMetrics?.messagesReceived ?? '-'}
          </div>
          <div className="text-sm text-gray-400">Messages Received</div>
        </div>
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
          <div className="text-2xl font-bold text-blue-400">
            {serverMetrics?.messagesSent ?? '-'}
          </div>
          <div className="text-sm text-gray-400">Messages Sent</div>
        </div>
      </div>

      {/* Connections Table */}
      <Card title="Connected Clients">
        <Table
          columns={columns}
          data={connections ?? []}
          keyExtractor={(conn) => conn.id}
          loading={loading}
          emptyMessage="No active connections"
        />
      </Card>
    </div>
  );
}
