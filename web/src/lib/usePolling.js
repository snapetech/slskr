import { useEffect, useRef } from 'react';

const documentIsHidden = () =>
  typeof document !== 'undefined' && document.visibilityState === 'hidden';

/**
 * Run an async refresh immediately and schedule the next refresh only after
 * the previous one completes. Hidden documents do not create background
 * request pressure; becoming visible triggers one fresh read.
 */
export const usePolling = (
  callback,
  intervalMilliseconds,
  {
    enabled = true,
    immediate = true,
    pauseWhenHidden = true,
    resetKey,
  } = {},
) => {
  const callbackRef = useRef(callback);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    if (
      !enabled ||
      !Number.isFinite(intervalMilliseconds) ||
      intervalMilliseconds <= 0
    ) {
      return undefined;
    }

    let cancelled = false;
    let running = false;
    let timerId;

    const schedule = (delay = intervalMilliseconds) => {
      if (cancelled) return;
      if (timerId !== undefined) globalThis.clearTimeout(timerId);
      timerId = globalThis.setTimeout(run, delay);
    };

    const run = async () => {
      timerId = undefined;
      if (cancelled) return;
      if (pauseWhenHidden && documentIsHidden()) {
        schedule();
        return;
      }
      if (running) {
        schedule();
        return;
      }

      running = true;
      try {
        await callbackRef.current();
      } catch (error) {
        if (!cancelled) {
          console.error('Polling callback failed:', error);
        }
      } finally {
        running = false;
        schedule();
      }
    };

    const handleVisibilityChange = () => {
      if (!documentIsHidden() && !running) schedule(0);
    };

    if (pauseWhenHidden && typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', handleVisibilityChange);
    }
    if (immediate) {
      void run();
    } else {
      schedule();
    }

    return () => {
      cancelled = true;
      if (timerId !== undefined) globalThis.clearTimeout(timerId);
      if (pauseWhenHidden && typeof document !== 'undefined') {
        document.removeEventListener(
          'visibilitychange',
          handleVisibilityChange,
        );
      }
    };
  }, [enabled, immediate, intervalMilliseconds, pauseWhenHidden, resetKey]);
};
