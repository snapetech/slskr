import React, { createContext, useCallback, useContext, useMemo, ReactNode } from 'react';
import { useLocalStorage, useSessionStorage } from '../hooks/useLocalStorage';
import { normalizeApiUrl } from '../lib/api';

interface ApiContextType {
  apiUrl: string;
  apiKey: string | null;
  isConnected: boolean;
  setApiUrl: (url: string) => void;
  setApiKey: (key: string | null) => void;
  setIsConnected: (connected: boolean) => void;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);
const DEFAULT_API_URL = 'http://127.0.0.1:5030';

interface ApiProviderProps {
  children: ReactNode;
}

/**
 * Provider for API configuration context
 * Eliminates prop drilling for apiUrl and apiKey
 */
export function ApiProvider({ children }: ApiProviderProps) {
  const [storedApiUrl, setStoredApiUrl] = useLocalStorage('apiUrl', DEFAULT_API_URL);
  const [apiKey, setApiKey] = useSessionStorage<string | null>('apiKey', null);
  const [isConnected, setIsConnected] = React.useState(false);
  const apiUrl = useMemo(() => {
    try {
      return normalizeApiUrl(storedApiUrl);
    } catch {
      return DEFAULT_API_URL;
    }
  }, [storedApiUrl]);
  const setApiUrl = useCallback(
    (url: string) => setStoredApiUrl(normalizeApiUrl(url)),
    [setStoredApiUrl],
  );

  const value: ApiContextType = {
    apiUrl,
    apiKey,
    isConnected,
    setApiUrl,
    setApiKey,
    setIsConnected,
  };

  return (
    <ApiContext.Provider value={value}>
      {children}
    </ApiContext.Provider>
  );
}

/**
 * Hook to use API context
 * Ensures context is used within provider
 */
export function useApi(): ApiContextType {
  const context = useContext(ApiContext);
  if (!context) {
    throw new Error('useApi must be used within ApiProvider');
  }
  return context;
}
