import { useState, useEffect, useRef, useCallback } from 'react';
import { readResponseText } from '../lib/api';

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

function headersKey(headers?: HeadersInit): string {
  if (!headers) return '';

  return JSON.stringify(
    Array.from(new Headers(headers).entries()).sort(([left], [right]) =>
      left.localeCompare(right),
    ),
  );
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
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const interval = options?.interval;
  const requestHeadersKey = headersKey(options?.headers);

  const fetchData = useCallback(async () => {
    if (!url) return;

    const currentOptions = optionsRef.current;

    // Cancel previous request
    abortControllerRef.current?.abort();
    const requestController = new AbortController();
    abortControllerRef.current = requestController;

    try {
      setLoading(true);
      setError(null);

      const response = await fetch(url, {
        signal: requestController.signal,
        headers: currentOptions?.headers || {},
        redirect: 'error',
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const body = await readResponseText(response);
      const result = body.trim() ? JSON.parse(body) as T : undefined as T;

      // Only update state if component is still mounted
      if (
        isMountedRef.current &&
        abortControllerRef.current === requestController
      ) {
        setData(result);
        setError(null);
      }
    } catch (err) {
      // Ignore abort errors (caused by cleanup or new request)
      if (err instanceof Error && err.name === 'AbortError') {
        return;
      }

      const error = err instanceof Error ? err : new Error('Unknown error');
      
      if (
        isMountedRef.current &&
        abortControllerRef.current === requestController
      ) {
        setError(error);
        setData(null);
        currentOptions?.onError?.(error);
      }
    } finally {
      if (
        isMountedRef.current &&
        abortControllerRef.current === requestController
      ) {
        setLoading(false);
      }
    }
  }, [url, requestHeadersKey]);

  useEffect(() => {
    isMountedRef.current = true;

    // Initial fetch
    fetchData();

    // Set up auto-refresh if interval is specified
    if (interval && interval > 0) {
      intervalRef.current = setInterval(fetchData, interval);
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
  }, [url, fetchData, interval]);

  return { data, loading, error, refetch: fetchData };
}
