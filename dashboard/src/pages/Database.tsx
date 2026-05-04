import React, { useState, useEffect } from 'react';
import { Trash2, Database } from 'lucide-react';

interface DatabasePageProps {
  apiUrl: string;
  apiKey: string | null;
}

interface DatabaseStats {
  search_count: number;
  transfer_count: number;
  message_count: number;
}

export default function Database({ apiUrl, apiKey }: DatabasePageProps) {
  const [stats, setStats] = useState<DatabaseStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchStats();
  }, [apiUrl, apiKey]);

  const fetchStats = async () => {
    try {
      setLoading(true);
      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/database/stats`, { headers });
      if (!res.ok) throw new Error('Failed to fetch database stats');
      const data = await res.json();
      setStats(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const handleCleanup = async (days: number) => {
    if (!window.confirm(`Delete records older than ${days} days?`)) return;

    try {
      const headers: HeadersInit = { 'Content-Type': 'application/json' };
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/database/cleanup`, {
        method: 'POST',
        headers,
        body: JSON.stringify({ days }),
      });

      if (!res.ok) throw new Error('Cleanup failed');
      alert('Cleanup completed');
      await fetchStats();
    } catch (err) {
      alert('Cleanup failed');
    }
  };

  const handleVacuum = async () => {
    if (!window.confirm('Optimize database? This may take time.')) return;

    try {
      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/database/vacuum`, {
        method: 'POST',
        headers,
      });

      if (!res.ok) throw new Error('Vacuum failed');
      alert('Vacuum completed');
    } catch (err) {
      alert('Vacuum failed');
    }
  };

  if (loading) return <div className="text-center text-gray-500">Loading...</div>;

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-gray-900">Database Management</h2>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}

      {/* Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-gray-600 text-sm">Searches</p>
              <p className="text-3xl font-bold text-gray-900 mt-2">{stats?.search_count || 0}</p>
            </div>
            <Database className="w-12 h-12 text-blue-100" />
          </div>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-gray-600 text-sm">Transfers</p>
              <p className="text-3xl font-bold text-gray-900 mt-2">{stats?.transfer_count || 0}</p>
            </div>
            <Database className="w-12 h-12 text-green-100" />
          </div>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-gray-600 text-sm">Messages</p>
              <p className="text-3xl font-bold text-gray-900 mt-2">{stats?.message_count || 0}</p>
            </div>
            <Database className="w-12 h-12 text-purple-100" />
          </div>
        </div>
      </div>

      {/* Maintenance */}
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Maintenance</h3>
        <div className="space-y-3">
          <button
            onClick={() => handleCleanup(30)}
            className="w-full px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 flex items-center gap-2"
          >
            <Trash2 className="w-4 h-4" />
            Cleanup Records (30+ days old)
          </button>
          <button
            onClick={() => handleCleanup(90)}
            className="w-full px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 flex items-center gap-2"
          >
            <Trash2 className="w-4 h-4" />
            Cleanup Records (90+ days old)
          </button>
          <button
            onClick={handleVacuum}
            className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Optimize Database
          </button>
        </div>
      </div>
    </div>
  );
}
