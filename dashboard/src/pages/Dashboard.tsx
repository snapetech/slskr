import React, { useState, useEffect } from 'react';
import { Activity, Users, Database, Zap } from 'lucide-react';

interface DashboardProps {
  apiUrl: string;
  apiKey: string | null;
}

interface ServerStats {
  total_searches: number;
  active_transfers: number;
  total_users: number;
  total_rooms: number;
  uptime: number;
}

interface HealthStatus {
  status: string;
  timestamp: number;
}

export default function Dashboard({ apiUrl, apiKey }: DashboardProps) {
  const [stats, setStats] = useState<ServerStats | null>(null);
  const [health, setHealth] = useState<HealthStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        setError(null);

        // Fetch health
        const healthRes = await fetch(`${apiUrl}/api/health`);
        if (!healthRes.ok) throw new Error('Failed to fetch health');
        const healthData = await healthRes.json();
        setHealth(healthData);

        // Fetch stats (requires auth if set)
        const headers: HeadersInit = {};
        if (apiKey) {
          headers['Authorization'] = `Bearer ${apiKey}`;
        }

        const statsRes = await fetch(`${apiUrl}/api/stats`, { headers });
        if (!statsRes.ok) throw new Error('Failed to fetch stats');
        const statsData = await statsRes.json();
        setStats(statsData);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 5000); // Refresh every 5s
    return () => clearInterval(interval);
  }, [apiUrl, apiKey]);

  if (loading && !stats) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-gray-500">Loading dashboard...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <p className="text-red-800">Error: {error}</p>
      </div>
    );
  }

  const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${mins}m`;
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-gray-900">Dashboard</h2>

      {/* Server Status */}
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Server Status</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="bg-blue-50 rounded-lg p-4">
            <p className="text-sm text-blue-600 font-medium">Status</p>
            <p className="text-2xl font-bold text-blue-900 mt-2">{health?.status || 'Unknown'}</p>
            <p className="text-xs text-blue-600 mt-2">
              {new Date(health?.timestamp ? health.timestamp * 1000 : 0).toLocaleString()}
            </p>
          </div>
          <div className="bg-green-50 rounded-lg p-4">
            <p className="text-sm text-green-600 font-medium">Uptime</p>
            <p className="text-2xl font-bold text-green-900 mt-2">
              {stats ? formatUptime(stats.uptime) : 'N/A'}
            </p>
          </div>
        </div>
      </div>

      {/* Statistics Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={Activity}
          label="Active Transfers"
          value={stats?.active_transfers || 0}
          color="blue"
        />
        <StatCard
          icon={Database}
          label="Total Searches"
          value={stats?.total_searches || 0}
          color="green"
        />
        <StatCard
          icon={Users}
          label="Total Users"
          value={stats?.total_users || 0}
          color="purple"
        />
        <StatCard
          icon={Zap}
          label="Total Rooms"
          value={stats?.total_rooms || 0}
          color="orange"
        />
      </div>

      {/* Quick Actions */}
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Quick Actions</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <QuickActionButton label="Refresh Stats" onClick={() => window.location.reload()} />
          <QuickActionButton label="View Logs" onClick={() => {}} disabled />
          <QuickActionButton label="Backup DB" onClick={() => {}} disabled />
          <QuickActionButton label="Health Check" onClick={() => {}} />
        </div>
      </div>

      {/* Auto-refresh indicator */}
      <div className="text-center text-xs text-gray-500">
        Auto-refreshing every 5 seconds...
      </div>
    </div>
  );
}

interface StatCardProps {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: number;
  color: 'blue' | 'green' | 'purple' | 'orange';
}

function StatCard({ icon: Icon, label, value, color }: StatCardProps) {
  const bgColors = {
    blue: 'bg-blue-50',
    green: 'bg-green-50',
    purple: 'bg-purple-50',
    orange: 'bg-orange-50',
  };

  const textColors = {
    blue: 'text-blue-600',
    green: 'text-green-600',
    purple: 'text-purple-600',
    orange: 'text-orange-600',
  };

  const valueColors = {
    blue: 'text-blue-900',
    green: 'text-green-900',
    purple: 'text-purple-900',
    orange: 'text-orange-900',
  };

  return (
    <div className={`${bgColors[color]} rounded-lg p-6`}>
      <div className="flex items-start justify-between">
        <div>
          <p className={`text-sm font-medium ${textColors[color]}`}>{label}</p>
          <p className={`text-3xl font-bold ${valueColors[color]} mt-2`}>{value}</p>
        </div>
        <Icon className={`w-8 h-8 ${textColors[color]} opacity-50`} />
      </div>
    </div>
  );
}

function QuickActionButton({
  label,
  onClick,
  disabled = false,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`px-4 py-2 rounded-lg font-medium transition-colors ${
        disabled
          ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
          : 'bg-blue-600 text-white hover:bg-blue-700'
      }`}
    >
      {label}
    </button>
  );
}
