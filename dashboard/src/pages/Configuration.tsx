import React, { useEffect, useState } from 'react';
import { Save } from 'lucide-react';
import { isAbortError, requestJson } from '../lib/api';

interface ConfigurationPageProps {
  apiUrl: string;
  apiKey: string | null;
}

interface Preferences {
  auto_connect?: boolean;
  transfer_allow_outbound?: boolean;
  transfer_max_active?: number;
  autoreplace_enabled?: boolean;
}

interface DownloadFilter {
  exclude?: string[];
  maxTerms?: number;
  maxTermLength?: number;
}

export default function Configuration({ apiUrl, apiKey }: ConfigurationPageProps) {
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const [exclusions, setExclusions] = useState('');
  const [limits, setLimits] = useState({ maxTerms: 100, maxTermLength: 256 });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<'preferences' | 'filter' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    let active = true;

    const loadConfiguration = async () => {
      setLoading(true);
      setError(null);
      try {
        const [nextPreferences, nextFilter] = await Promise.all([
          requestJson<Preferences>(`${apiUrl}/api/config/preferences`, apiKey, {
            signal: controller.signal,
          }),
          requestJson<DownloadFilter>(`${apiUrl}/api/config/download-filter`, apiKey, {
            signal: controller.signal,
          }),
        ]);
        if (!active) return;
        setPreferences(nextPreferences);
        setExclusions((nextFilter.exclude ?? []).join('\n'));
        setLimits({
          maxTerms: nextFilter.maxTerms ?? 100,
          maxTermLength: nextFilter.maxTermLength ?? 256,
        });
      } catch (loadError) {
        if (!active || isAbortError(loadError)) return;
        setError(loadError instanceof Error ? loadError.message : 'Failed to load configuration');
      } finally {
        if (active) setLoading(false);
      }
    };

    void loadConfiguration();
    return () => {
      active = false;
      controller.abort();
    };
  }, [apiUrl, apiKey]);

  const savePreferences = async () => {
    if (!preferences) return;
    setSaving('preferences');
    setError(null);
    setMessage(null);
    try {
      await requestJson(`${apiUrl}/api/config/preferences`, apiKey, {
        method: 'PUT',
        body: JSON.stringify({
          autoreplace_enabled: Boolean(preferences.autoreplace_enabled),
        }),
      });
      setMessage('Runtime preferences saved.');
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Failed to save preferences');
    } finally {
      setSaving(null);
    }
  };

  const saveFilter = async () => {
    setSaving('filter');
    setError(null);
    setMessage(null);
    try {
      const exclude = exclusions
        .split('\n')
        .map((term) => term.trim())
        .filter(Boolean);
      await requestJson(`${apiUrl}/api/config/download-filter`, apiKey, {
        method: 'PUT',
        body: JSON.stringify({ exclude }),
      });
      setExclusions(exclude.join('\n'));
      setMessage('Download filter saved.');
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : 'Failed to save download filter');
    } finally {
      setSaving(null);
    }
  };

  if (loading) return <div className="text-center text-gray-500">Loading configuration...</div>;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900">Configuration</h2>
        <p className="mt-1 text-sm text-gray-600">
          Runtime preferences and download policy are the settings exposed by the daemon API.
          Startup-only values remain read-only here.
        </p>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}
      {message && (
        <div className="bg-green-50 border border-green-200 rounded-lg p-4">
          <p className="text-green-800">{message}</p>
        </div>
      )}

      <section className="bg-white rounded-lg shadow p-6 space-y-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Runtime preferences</h3>
          <p className="text-sm text-gray-600 mt-1">
            The autoreplace switch can be changed without restarting the daemon.
          </p>
        </div>
        <label className="flex items-center gap-3 text-gray-700">
          <input
            type="checkbox"
            checked={Boolean(preferences?.autoreplace_enabled)}
            onChange={(event) => setPreferences((current) => current
              ? { ...current, autoreplace_enabled: event.target.checked }
              : current)}
            className="w-4 h-4"
          />
          Enable automatic replacement
        </label>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <ReadOnlyValue label="Auto-connect" value={preferences?.auto_connect ? 'Enabled' : 'Disabled'} />
          <ReadOnlyValue
            label="Outbound transfers"
            value={preferences?.transfer_allow_outbound ? 'Enabled' : 'Disabled'}
          />
          <ReadOnlyValue
            label="Max active transfers"
            value={preferences?.transfer_max_active ?? 'Unknown'}
          />
        </div>
        <button
          onClick={savePreferences}
          disabled={!preferences || saving !== null}
          className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50"
        >
          <Save className="w-4 h-4" />
          {saving === 'preferences' ? 'Saving...' : 'Save preferences'}
        </button>
      </section>

      <section className="bg-white rounded-lg shadow p-6 space-y-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">Download filter</h3>
          <p className="text-sm text-gray-600 mt-1">
            One exclusion term per line. The daemon accepts at most {limits.maxTerms} terms,
            with a maximum length of {limits.maxTermLength} characters each.
          </p>
        </div>
        <textarea
          value={exclusions}
          onChange={(event) => setExclusions(event.target.value)}
          rows={8}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          placeholder="term to exclude\nanother term"
        />
        <button
          onClick={saveFilter}
          disabled={saving !== null}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
        >
          <Save className="w-4 h-4" />
          {saving === 'filter' ? 'Saving...' : 'Save download filter'}
        </button>
      </section>
    </div>
  );
}

function ReadOnlyValue({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-lg bg-gray-50 p-3">
      <p className="text-gray-500">{label}</p>
      <p className="font-medium text-gray-900 mt-1">{value}</p>
    </div>
  );
}
