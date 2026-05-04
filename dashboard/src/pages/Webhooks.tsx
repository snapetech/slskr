import React, { useState, useEffect } from 'react';
import { Plus, Trash2, TestTube } from 'lucide-react';

interface WebhooksPageProps {
  apiUrl: string;
  apiKey: string | null;
}

interface Webhook {
  id: string;
  url: string;
  events: string[];
  active: boolean;
  created_at: number;
  last_triggered?: number;
}

export default function Webhooks({ apiUrl, apiKey }: WebhooksPageProps) {
  const [webhooks, setWebhooks] = useState<Webhook[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [newUrl, setNewUrl] = useState('');
  const [selectedEvents, setSelectedEvents] = useState<string[]>(['search.created']);

  const availableEvents = [
    'search.created',
    'search.completed',
    'transfer.started',
    'transfer.completed',
    'message.received',
    'message.sent',
    'user.connected',
    'user.disconnected',
  ];

  useEffect(() => {
    fetchWebhooks();
  }, [apiUrl, apiKey]);

  const fetchWebhooks = async () => {
    try {
      setLoading(true);
      setError(null);

      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/webhooks`, { headers });
      if (!res.ok) throw new Error('Failed to fetch webhooks');
      const data = await res.json();
      setWebhooks(Array.isArray(data) ? data : data.webhooks || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const handleCreateWebhook = async () => {
    if (!newUrl) return;

    try {
      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      };
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/webhooks`, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          url: newUrl,
          events: selectedEvents,
        }),
      });

      if (!res.ok) throw new Error('Failed to create webhook');
      
      setShowForm(false);
      setNewUrl('');
      setSelectedEvents(['search.created']);
      await fetchWebhooks();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  const handleDeleteWebhook = async (id: string) => {
    if (!window.confirm('Delete this webhook?')) return;

    try {
      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      const res = await fetch(`${apiUrl}/api/admin/webhooks/${id}`, {
        method: 'DELETE',
        headers,
      });

      if (!res.ok) throw new Error('Failed to delete webhook');
      await fetchWebhooks();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  const handleTestWebhook = async (id: string) => {
    try {
      const headers: HeadersInit = {};
      if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;

      await fetch(`${apiUrl}/api/admin/webhooks/${id}/test`, {
        method: 'POST',
        headers,
      });

      alert('Test webhook sent!');
    } catch (err) {
      alert('Failed to test webhook');
    }
  };

  if (loading) return <div className="text-center text-gray-500">Loading webhooks...</div>;

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold text-gray-900">Webhooks</h2>
        <button
          onClick={() => setShowForm(!showForm)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          <Plus className="w-4 h-4" />
          New Webhook
        </button>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}

      {showForm && (
        <div className="bg-white rounded-lg shadow p-6">
          <h3 className="text-lg font-semibold mb-4">Create Webhook</h3>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">URL</label>
              <input
                type="url"
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="https://example.com/webhook"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">Events</label>
              <div className="grid grid-cols-2 gap-2">
                {availableEvents.map((event) => (
                  <label key={event} className="flex items-center">
                    <input
                      type="checkbox"
                      checked={selectedEvents.includes(event)}
                      onChange={(e) => {
                        if (e.target.checked) {
                          setSelectedEvents([...selectedEvents, event]);
                        } else {
                          setSelectedEvents(selectedEvents.filter((s) => s !== event));
                        }
                      }}
                      className="w-4 h-4 text-blue-600 rounded"
                    />
                    <span className="ml-2 text-gray-700 text-sm">{event}</span>
                  </label>
                ))}
              </div>
            </div>

            <div className="flex gap-2">
              <button
                onClick={handleCreateWebhook}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                Create
              </button>
              <button
                onClick={() => setShowForm(false)}
                className="px-4 py-2 bg-gray-300 text-gray-700 rounded-lg hover:bg-gray-400"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Webhooks List */}
      <div className="space-y-3">
        {webhooks.length === 0 ? (
          <div className="bg-white rounded-lg p-6 text-center text-gray-500">
            No webhooks configured
          </div>
        ) : (
          webhooks.map((webhook) => (
            <div key={webhook.id} className="bg-white rounded-lg shadow p-4">
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <p className="font-medium text-gray-900">{webhook.url}</p>
                  <div className="flex flex-wrap gap-1 mt-2">
                    {webhook.events.map((event) => (
                      <span key={event} className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded">
                        {event}
                      </span>
                    ))}
                  </div>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => handleTestWebhook(webhook.id)}
                    className="p-2 hover:bg-blue-100 rounded text-blue-600"
                  >
                    <TestTube className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => handleDeleteWebhook(webhook.id)}
                    className="p-2 hover:bg-red-100 rounded text-red-600"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
