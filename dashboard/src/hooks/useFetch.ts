import { useState, useEffect, useRef, useCallback } from 'react';

interface UseFetchOptions {
  headers?: HeadersInit;
  interval?: number; // Auto-refresh interval in ms
  onError?: (error: Error) => void;
}

interface UseFetchState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
}

/**
 * Custom hook for fetching data with proper cleanup and abort handling
 */
export function useFetch<T>(
  url: string | null,
  options?: UseFetchOptions
): UseFetchState<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(!!url);
  const [error, setError] = useState<Error | null>(null);
  
  // Use ref to track if component is mounted
  const isMountedRef = useRef(true);
  const abortControllerRef = useRef<AbortController | null>(null);
  const intervalRef = useRef<NodeJS.Timer | null>(null);

  const fetchData = useCallback(async () => {
    if (!url) return;

    // Cancel previous request
    abortControllerRef.current?.abort();
    abortControllerRef.current = new AbortController();

    try {
      setLoading(true);
      setError(null);

      const response = await fetch(url, {
        signal: abortControllerRef.current.signal,
        headers: options?.headers || {},
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result = await response.json() as T;

      // Only update state if component is still mounted
      if (isMountedRef.current) {
        setData(result);
        setError(null);
      }
    } catch (err) {
      // Ignore abort errors (caused by cleanup or new request)
      if (err instanceof Error && err.name === 'AbortError') {
        return;
      }

      const error = err instanceof Error ? err : new Error('Unknown error');
      
      if (isMountedRef.current) {
        setError(error);
        setData(null);
        options?.onError?.(error);
      }
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, [url, options]);

  useEffect(() => {
    isMountedRef.current = true;

    // Initial fetch
    fetchData();

    // Set up auto-refresh if interval is specified
    if (options?.interval && options.interval > 0) {
      intervalRef.current = setInterval(fetchData, options.interval);
    }

    // Cleanup on unmount
    return () => {
      isMountedRef.current = false;
      
      // Abort any pending requests
      abortControllerRef.current?.abort();
      
      // Clear interval
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [url, fetchData, options?.interval]);

  return { data, loading, error, refetch: fetchData };
}
