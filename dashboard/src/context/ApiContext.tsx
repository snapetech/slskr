import React, { createContext, useContext, ReactNode } from 'react';
import { useLocalStorage } from '../hooks/useLocalStorage';

interface ApiContextType {
  apiUrl: string;
  apiKey: string | null;
  isConnected: boolean;
  setApiUrl: (url: string) => void;
  setApiKey: (key: string | null) => void;
  setIsConnected: (connected: boolean) => void;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);

interface ApiProviderProps {
  children: ReactNode;
}

/**
 * Provider for API configuration context
 * Eliminates prop drilling for apiUrl and apiKey
 */
export function ApiProvider({ children }: ApiProviderProps) {
  const [apiUrl, setApiUrl] = useLocalStorage('apiUrl', 'http://localhost:8080');
  const [apiKey, setApiKey] = useLocalStorage<string | null>('apiKey', null);
  const [isConnected, setIsConnected] = React.useState(false);

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
