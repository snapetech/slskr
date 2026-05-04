import React, { useState } from 'react';
import { Server, Settings, LogOut } from 'lucide-react';

interface HeaderProps {
  isConnected: boolean;
  apiUrl: string;
  onApiUrlChange: (url: string) => void;
  onApiKeyChange: (key: string) => void;
}

export default function Header({
  isConnected,
  apiUrl,
  onApiUrlChange,
  onApiKeyChange,
}: HeaderProps) {
  const [showSettings, setShowSettings] = useState(false);
  const [tempUrl, setTempUrl] = useState(apiUrl);
  const [tempKey, setTempKey] = useState('');

  const handleSave = () => {
    if (tempUrl) onApiUrlChange(tempUrl);
    if (tempKey) onApiKeyChange(tempKey);
    setShowSettings(false);
    setTempKey('');
  };

  return (
    <header className="bg-white shadow-sm border-b border-gray-200">
      <div className="flex justify-between items-center px-8 py-4">
        <div className="flex items-center gap-3">
          <Server className="w-6 h-6 text-blue-600" />
          <h1 className="text-xl font-semibold text-gray-900">soulseekR Admin</h1>
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
            isConnected
              ? 'bg-green-100 text-green-800'
              : 'bg-red-100 text-red-800'
          }`}>
            {isConnected ? '✓ Connected' : '✗ Disconnected'}
          </span>
        </div>

        <button
          onClick={() => setShowSettings(!showSettings)}
          className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
        >
          <Settings className="w-5 h-5 text-gray-600" />
        </button>
      </div>

      {showSettings && (
        <div className="px-8 py-4 bg-gray-50 border-t border-gray-200">
          <div className="space-y-4 max-w-md">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                API URL
              </label>
              <input
                type="text"
                value={tempUrl}
                onChange={(e) => setTempUrl(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="http://localhost:8080"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                API Key (optional)
              </label>
              <input
                type="password"
                value={tempKey}
                onChange={(e) => setTempKey(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Leave empty to use stored key"
              />
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSave}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                Save
              </button>
              <button
                onClick={() => setShowSettings(false)}
                className="px-4 py-2 bg-gray-300 text-gray-700 rounded-lg hover:bg-gray-400 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </header>
  );
}
