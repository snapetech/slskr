import React, { useState, useEffect } from 'react';
import { RefreshCw } from 'lucide-react';

interface MonitoringPageProps {
  apiUrl: string;
  apiKey: string | null;
}

interface Metrics {
  requests_total: number;
  requests_error: number;
  latency_p95: number;
  latency_p99: number;
}

export default function Monitoring({ apiUrl, apiKey }: MonitoringPageProps) {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        setLoading(true);
        const headers: HeadersInit = {};
        if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

        const res = await fetch(`${apiUrl}/api/metrics`, { headers });
        if (!res.ok) throw new Error('Failed to fetch metrics');
        const data = await res.json();
        setMetrics(data);
      } catch (err) {
        console.error('Failed to fetch metrics:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchMetrics();
    if (autoRefresh) {
      const interval = setInterval(fetchMetrics, 5000);
      return () => clearInterval(interval);
    }
  }, [apiUrl, apiKey, autoRefresh]);

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-900">Performance Monitoring</h2>
        <label className="flex items-center gap-2 text-gray-700">
          <input
            type="checkbox"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
            className="w-4 h-4"
          />
          Auto-refresh every 5s
        </label>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Total Requests</p>
          <p className="text-4xl font-bold text-blue-600 mt-2">{metrics?.requests_total || 0}</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Error Rate</p>
          <p className="text-4xl font-bold text-red-600 mt-2">
            {metrics ? ((metrics.requests_error / metrics.requests_total) * 100).toFixed(2) : 0}%
          </p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">P95 Latency</p>
          <p className="text-4xl font-bold text-green-600 mt-2">{metrics?.latency_p95 || 0}ms</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">P99 Latency</p>
          <p className="text-4xl font-bold text-orange-600 mt-2">{metrics?.latency_p99 || 0}ms</p>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Prometheus Metrics</h3>
        <p className="text-gray-600 mb-4">
          Raw Prometheus metrics are available at: <code className="bg-gray-100 px-2 py-1 rounded text-sm">/metrics</code>
        </p>
        <a
          href={`${apiUrl}/metrics`}
          target="_blank"
          rel="noopener noreferrer"
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          View Metrics →
        </a>
      </div>
    </div>
  );
}
