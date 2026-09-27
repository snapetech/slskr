import React, { lazy, Suspense, useEffect, useMemo } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { ErrorBoundary } from './components/ErrorBoundary';
import { ApiProvider, useApi } from './context/ApiContext';
import { useFetch } from './hooks/useFetch';
import Sidebar from './components/Sidebar';
import Header from './components/Header';
import { apiEndpoint } from './lib/api';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const ApiKeys = lazy(() => import('./pages/ApiKeys'));
const Webhooks = lazy(() => import('./pages/Webhooks'));
const Database = lazy(() => import('./pages/Database'));
const Monitoring = lazy(() => import('./pages/Monitoring'));
const Configuration = lazy(() => import('./pages/Configuration'));

/**
 * Inner app component that uses ApiContext
 * Separated to ensure context is available
 */
function AppContent() {
  const { apiUrl, apiKey, isConnected, setIsConnected } = useApi();

  const healthHeaders = useMemo<HeadersInit>(() => {
    const headers: HeadersInit = {};
    if (apiKey) headers['Authorization'] = `Bearer ${apiKey}`;
    return headers;
  }, [apiKey]);

  // Fetch health check with auto-refresh
  const { data: health, error: healthError } = useFetch(
    apiEndpoint(apiUrl, '/api/health'),
    { headers: healthHeaders, interval: 5000 }
  );

  // Update connection status when health data changes
  useEffect(() => {
    // Keep the last known-good session usable during a transient refresh
    // failure. The banner below exposes the degraded state without replacing
    // the whole application with the initial connection screen.
    setIsConnected(!!health);
  }, [health, setIsConnected]);

  return (
    <Router>
      <div className="flex h-screen bg-gray-50">
        <Sidebar />
        <div className="flex flex-col flex-1">
          <Header />
          <main className="flex-1 overflow-auto p-8">
            {!isConnected ? (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <h1 className="text-3xl font-bold text-gray-900 mb-4">
                    Connection Required
                  </h1>
                  <p className="text-gray-600 mb-6">
                    Unable to connect to API
                  </p>
                  <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-left">
                    <p className="text-sm text-blue-800 font-mono">
                      API URL: {apiUrl}
                    </p>
                    <p className="text-sm text-blue-800 font-mono mt-2" role="status" aria-live="polite">
                      Status: {healthError ? `Unavailable: ${healthError.message}` : 'Checking...'}
                    </p>
                  </div>
                </div>
              </div>
            ) : (
              <>
                {healthError && (
                  <div
                    className="mb-4 rounded-lg border border-yellow-200 bg-yellow-50 p-4"
                    role="alert"
                    aria-live="polite"
                  >
                    <p className="text-sm text-yellow-800">
                      Connection degraded: {healthError.message}. Showing the
                      last known-good dashboard state while retrying.
                    </p>
                  </div>
                )}
                <Suspense fallback={<div role="status">Loading dashboard...</div>}>
                  <Routes>
                    <Route path="/" element={<Dashboard />} />
                    <Route path="/api-keys" element={<ApiKeys apiUrl={apiUrl} apiKey={apiKey} />} />
                    <Route path="/webhooks" element={<Webhooks apiUrl={apiUrl} apiKey={apiKey} />} />
                    <Route path="/database" element={<Database apiUrl={apiUrl} apiKey={apiKey} />} />
                    <Route path="/monitoring" element={<Monitoring apiUrl={apiUrl} apiKey={apiKey} />} />
                    <Route path="/configuration" element={<Configuration apiUrl={apiUrl} apiKey={apiKey} />} />
                    <Route path="*" element={<Navigate to="/" replace />} />
                  </Routes>
                </Suspense>
              </>
            )}
          </main>
        </div>
      </div>
    </Router>
  );
}

/**
 * Root App component with error boundary and context providers
 */
export default function App() {
  return (
    <ErrorBoundary>
      <ApiProvider>
        <AppContent />
      </ApiProvider>
    </ErrorBoundary>
  );
}
