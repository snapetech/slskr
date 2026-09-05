import React, { useState, useEffect } from 'react';
import { apiEndpoint, isAbortError, requestText } from '../lib/api';
import { parseMonitoringMetrics, type MonitoringMetrics } from '../lib/prometheus';

interface MonitoringPageProps {
  apiUrl: string;
  apiKey: string | null;
}

export default function Monitoring({ apiUrl, apiKey }: MonitoringPageProps) {
  const [metrics, setMetrics] = useState<MonitoringMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [downloadingMetrics, setDownloadingMetrics] = useState(false);
  const metricsUrl = apiEndpoint(apiUrl, '/api/metrics');

  useEffect(() => {
    let active = true;
    let controller: AbortController | undefined;

    const fetchMetrics = async () => {
      controller?.abort();
      const requestController = new AbortController();
      controller = requestController;

      try {
        setLoading(true);
        setError(null);
        const nextMetrics = parseMonitoringMetrics(
          await requestText(metricsUrl, apiKey, { signal: requestController.signal }),
        );
        if (active && controller === requestController) {
          setMetrics(nextMetrics);
        }
      } catch (err) {
        if (
          !active ||
          controller !== requestController ||
          isAbortError(err)
        ) {
          return;
        }
        console.error('Failed to fetch metrics:', err);
        setError(err instanceof Error ? err.message : 'Failed to fetch metrics');
      } finally {
        if (active && controller === requestController) {
          setLoading(false);
        }
      }
    };

    fetchMetrics();
    if (autoRefresh) {
      const interval = setInterval(fetchMetrics, 5000);
      return () => {
        active = false;
        controller?.abort();
        clearInterval(interval);
      };
    }

    return () => {
      active = false;
      controller?.abort();
    };
  }, [apiKey, autoRefresh, metricsUrl]);

  const downloadMetrics = async () => {
    setDownloadingMetrics(true);
    try {
      const body = await requestText(metricsUrl, apiKey);
      const objectUrl = URL.createObjectURL(
        new Blob([body], { type: 'text/plain;charset=utf-8' }),
      );
      const link = document.createElement('a');
      link.href = objectUrl;
      link.download = 'slskr-metrics.txt';
      link.rel = 'noopener';
      link.click();
      window.setTimeout(() => URL.revokeObjectURL(objectUrl), 60_000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to download metrics');
    } finally {
      setDownloadingMetrics(false);
    }
  };

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

      {loading && !metrics && <p className="text-gray-500">Loading monitoring metrics…</p>}
      {error && <p className="text-red-700">{error}</p>}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Shared Files</p>
          <p className="text-4xl font-bold text-blue-600 mt-2">{metrics?.sharesFiles ?? 0}</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Total Transfers</p>
          <p className="text-4xl font-bold text-purple-600 mt-2">{metrics?.transfersTotal ?? 0}</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Active Transfers</p>
          <p className="text-4xl font-bold text-green-600 mt-2">{metrics?.transfersActive ?? 0}</p>
        </div>

        <div className="bg-white rounded-lg shadow p-6">
          <p className="text-gray-600 text-sm">Recorded Events</p>
          <p className="text-4xl font-bold text-orange-600 mt-2">{metrics?.eventsTotal ?? 0}</p>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Prometheus Metrics</h3>
        <p className="text-gray-600 mb-4">
          Raw Prometheus metrics are available at:{' '}
          <code className="bg-gray-100 px-2 py-1 rounded text-sm">/api/metrics</code>
        </p>
        <button
          type="button"
          onClick={downloadMetrics}
          disabled={downloadingMetrics}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          {downloadingMetrics ? 'Preparing Metrics…' : 'Download Raw Metrics ↓'}
        </button>
      </div>
    </div>
  );
}
