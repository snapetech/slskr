import { useState, useEffect, useRef, useCallback } from 'react';
import { isAbortError, readResponseText } from '../lib/api';

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
  const requestRef = useRef<Promise<void> | null>(null);
  const intervalRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const interval = options?.interval;
  const requestHeadersKey = headersKey(options?.headers);

  const fetchData = useCallback(async () => {
    if (!url) {
      abortControllerRef.current?.abort();
      abortControllerRef.current = null;
      requestRef.current = null;
      if (intervalRef.current) {
        clearTimeout(intervalRef.current);
        intervalRef.current = null;
      }
      if (isMountedRef.current) {
        setData(null);
        setError(null);
        setLoading(false);
      }
      return;
    }

    // A manual refresh supersedes a poll that has not fired yet. Clearing the
    // timeout before checking requestRef also makes repeated manual refreshes
    // unable to leave duplicate timers behind.
    if (intervalRef.current !== null) {
      clearTimeout(intervalRef.current);
      intervalRef.current = null;
    }

    if (requestRef.current) {
      return requestRef.current;
    }

    const currentOptions = optionsRef.current;

    const requestController = new AbortController();
    abortControllerRef.current = requestController;

    let request: Promise<void> | null = null;
    request = (async () => {
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

        if (
          isMountedRef.current &&
          abortControllerRef.current === requestController
        ) {
          setData(result);
          setError(null);
        }
      } catch (err) {
        if (isAbortError(err)) {
          return;
        }

        const error = err instanceof Error ? err : new Error('Unknown error');

        if (
          isMountedRef.current &&
          abortControllerRef.current === requestController
        ) {
          setError(error);
          currentOptions?.onError?.(error);
        }
      } finally {
        if (
          isMountedRef.current &&
          abortControllerRef.current === requestController
        ) {
          setLoading(false);
        }
        if (requestRef.current === request) {
          requestRef.current = null;
          if (
            isMountedRef.current &&
            interval &&
            interval > 0 &&
            document.visibilityState !== 'hidden'
          ) {
            intervalRef.current = setTimeout(() => {
              intervalRef.current = null;
              void fetchData();
            }, interval);
          }
        }
      }
    })();
    requestRef.current = request;
    return request;
  }, [url, requestHeadersKey, interval]);

  useEffect(() => {
    isMountedRef.current = true;

    void fetchData();

    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        if (intervalRef.current !== null) {
          clearTimeout(intervalRef.current);
          intervalRef.current = null;
        }
        return;
      }

      if (!requestRef.current) {
        if (intervalRef.current !== null) {
          clearTimeout(intervalRef.current);
          intervalRef.current = null;
        }
        void fetchData();
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Cleanup on unmount
    return () => {
      isMountedRef.current = false;
      abortControllerRef.current?.abort();
      abortControllerRef.current = null;
      requestRef.current = null;
      if (intervalRef.current) {
        clearTimeout(intervalRef.current);
        intervalRef.current = null;
      }
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [url, fetchData, interval]);

  return { data, loading, error, refetch: fetchData };
}
