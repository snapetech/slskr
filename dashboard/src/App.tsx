import React, { useState, useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import Sidebar from './components/Sidebar';
import Header from './components/Header';
import Dashboard from './pages/Dashboard';
import ApiKeys from './pages/ApiKeys';
import Webhooks from './pages/Webhooks';
import Database from './pages/Database';
import Monitoring from './pages/Monitoring';
import Configuration from './pages/Configuration';

interface AppState {
  isConnected: boolean;
  apiUrl: string;
  apiKey: string | null;
}

export default function App() {
  const [state, setState] = useState<AppState>({
    isConnected: false,
    apiUrl: localStorage.getItem('apiUrl') || 'http://localhost:8080',
    apiKey: localStorage.getItem('apiKey') || null,
  });

  useEffect(() => {
    // Check connection on mount
    checkConnection();
  }, [state.apiUrl]);

  const checkConnection = async () => {
    try {
      const response = await fetch(`${state.apiUrl}/api/health`);
      setState(prev => ({
        ...prev,
        isConnected: response.ok
      }));
    } catch (error) {
      setState(prev => ({
        ...prev,
        isConnected: false
      }));
    }
  };

  const handleSetApiKey = (key: string) => {
    localStorage.setItem('apiKey', key);
    setState(prev => ({
      ...prev,
      apiKey: key
    }));
  };

  const handleSetApiUrl = (url: string) => {
    localStorage.setItem('apiUrl', url);
    setState(prev => ({
      ...prev,
      apiUrl: url
    }));
    checkConnection();
  };

  return (
    <Router>
      <div className="flex h-screen bg-gray-50">
        <Sidebar />
        <div className="flex flex-col flex-1">
          <Header 
            isConnected={state.isConnected}
            apiUrl={state.apiUrl}
            onApiUrlChange={handleSetApiUrl}
            onApiKeyChange={handleSetApiKey}
          />
          <main className="flex-1 overflow-auto p-8">
            {!state.isConnected ? (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <h1 className="text-3xl font-bold text-gray-900 mb-4">
                    Connection Required
                  </h1>
                  <p className="text-gray-600 mb-6">
                    Unable to connect to {state.apiUrl}
                  </p>
                  <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-left">
                    <p className="text-sm text-blue-800 font-mono">
                      API URL: {state.apiUrl}
                    </p>
                  </div>
                </div>
              </div>
            ) : (
              <Routes>
                <Route path="/" element={<Dashboard apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="/api-keys" element={<ApiKeys apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="/webhooks" element={<Webhooks apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="/database" element={<Database apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="/monitoring" element={<Monitoring apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="/configuration" element={<Configuration apiUrl={state.apiUrl} apiKey={state.apiKey} />} />
                <Route path="*" element={<Navigate to="/" />} />
              </Routes>
            )}
          </main>
        </div>
      </div>
    </Router>
  );
}
