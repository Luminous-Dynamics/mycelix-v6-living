import { Card, Badge } from '../components';
import { useApi } from '../hooks/useApi';
import { apiClient } from '../api/client';

export default function Settings() {
  const { data: config, loading, error } = useApi(() => apiClient.getConfig(), []);

  if (loading) {
    return (
      <div className="p-6">
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-mycelix-500"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <div className="bg-red-900/30 border border-red-700 rounded-lg p-4 text-red-400">
          Failed to load configuration: {error}
        </div>
      </div>
    );
  }

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white">Server Configuration</h1>
        <p className="text-gray-400 mt-1">Current server settings (read-only)</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Network Settings */}
        <Card title="Network">
          <div className="space-y-4">
            <ConfigItem
              label="WebSocket Address"
              value={config?.bindAddr ?? '-'}
            />
            <ConfigItem
              label="Health/Metrics Address"
              value={config?.healthAddr ?? 'Disabled'}
            />
          </div>
        </Card>

        {/* Connection Limits */}
        <Card title="Connection Limits">
          <div className="space-y-4">
            <ConfigItem
              label="Max Total Connections"
              value={config?.maxConnections?.toString() ?? '-'}
            />
            <ConfigItem
              label="Max Connections Per IP"
              value={config?.maxConnectionsPerIp?.toString() ?? '-'}
            />
          </div>
        </Card>

        {/* Rate Limiting */}
        <Card title="Rate Limiting">
          <div className="space-y-4">
            <ConfigItem
              label="Requests Per Second"
              value={config?.rateLimit?.toString() ?? '-'}
            />
            <ConfigItem
              label="Burst Size"
              value={config?.rateLimitBurst?.toString() ?? '-'}
            />
          </div>
        </Card>

        {/* Security */}
        <Card title="Security">
          <div className="space-y-4">
            <ConfigItem
              label="Authentication Required"
              value={
                <Badge variant={config?.authRequired ? 'success' : 'warning'}>
                  {config?.authRequired ? 'Yes' : 'No'}
                </Badge>
              }
            />
            <ConfigItem
              label="Test Mode"
              value={
                <Badge variant={config?.testMode ? 'warning' : 'success'}>
                  {config?.testMode ? 'Enabled' : 'Disabled'}
                </Badge>
              }
            />
          </div>
        </Card>
      </div>

      {/* Environment Info */}
      <Card title="Environment" className="mt-6">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center">
            <div className="text-lg font-bold text-white">0.6.0</div>
            <div className="text-xs text-gray-400">Version</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-white">Rust</div>
            <div className="text-xs text-gray-400">Runtime</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-white">Tokio</div>
            <div className="text-xs text-gray-400">Async Runtime</div>
          </div>
          <div className="text-center">
            <div className="text-lg font-bold text-white">28 Days</div>
            <div className="text-xs text-gray-400">Cycle Duration</div>
          </div>
        </div>
      </Card>

      {/* API Information */}
      <Card title="API Endpoints" className="mt-6">
        <div className="space-y-2 font-mono text-sm">
          <div className="flex justify-between text-gray-300">
            <span>GET /admin/api/state</span>
            <span className="text-gray-500">Current cycle state</span>
          </div>
          <div className="flex justify-between text-gray-300">
            <span>GET /admin/api/connections</span>
            <span className="text-gray-500">Active connections</span>
          </div>
          <div className="flex justify-between text-gray-300">
            <span>GET /admin/api/metrics</span>
            <span className="text-gray-500">Phase metrics</span>
          </div>
          <div className="flex justify-between text-gray-300">
            <span>GET /admin/api/history</span>
            <span className="text-gray-500">Transition history</span>
          </div>
          <div className="flex justify-between text-gray-300">
            <span>GET /admin/api/config</span>
            <span className="text-gray-500">Server configuration</span>
          </div>
          <div className="flex justify-between text-gray-300">
            <span>POST /admin/api/cycle/advance</span>
            <span className="text-gray-500">Advance phase (test mode)</span>
          </div>
        </div>
      </Card>
    </div>
  );
}

interface ConfigItemProps {
  label: string;
  value: React.ReactNode;
}

function ConfigItem({ label, value }: ConfigItemProps) {
  return (
    <div className="flex justify-between items-center">
      <span className="text-gray-400">{label}</span>
      <span className="text-white font-medium">{value}</span>
    </div>
  );
}
