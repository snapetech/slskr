import React, { useState, useEffect } from 'react';
import { Save } from 'lucide-react';

interface ConfigurationPageProps {
  apiUrl: string;
  apiKey: string | null;
}

interface Config {
  [key: string]: any;
}

export default function Configuration({ apiUrl, apiKey }: ConfigurationPageProps) {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [changed, setChanged] = useState(false);

  useEffect(() => {
    fetchConfig();
  }, [apiUrl, apiKey]);

  const fetchConfig = async () => {
    try {
      setLoading(true);
      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/config`, { headers });
      if (!res.ok) throw new Error('Failed to fetch config');
      const data = await res.json();
      setConfig(data);
      setChanged(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!config) return;

    try {
      const headers: HeadersInit = { 'Content-Type': 'application/json' };
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/config`, {
        method: 'POST',
        headers,
        body: JSON.stringify(config),
      });

      if (!res.ok) throw new Error('Failed to save config');
      alert('Configuration saved successfully');
      setChanged(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  if (loading) return <div className="text-center text-gray-500">Loading...</div>;

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-900">Configuration</h2>
        {changed && (
          <button
            onClick={handleSave}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700"
          >
            <Save className="w-4 h-4" />
            Save Changes
          </button>
        )}
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}

      <div className="bg-white rounded-lg shadow p-6">
        <div className="space-y-4">
          {config && Object.entries(config).map(([key, value]) => (
            <div key={key}>
              <label className="block text-sm font-medium text-gray-700 mb-1">{key}</label>
              <input
                type="text"
                value={String(value)}
                onChange={(e) => {
                  setConfig({ ...config, [key]: e.target.value });
                  setChanged(true);
                }}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
