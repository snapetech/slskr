import { useEffect, useRef } from 'react';

const documentIsHidden = () =>
  typeof document !== 'undefined' && document.visibilityState === 'hidden';

/**
 * Create a lifecycle-controlled poller for class components and other code
 * that cannot call React hooks. A refresh never overlaps its predecessor, and
 * the controller owns the visibility listener and timer cleanup.
 */
export const createPollingController = (
  callback,
  intervalMilliseconds,
  { immediate = true, pauseWhenHidden = true, onError } = {},
) => {
  if (
    !Number.isFinite(intervalMilliseconds) ||
    intervalMilliseconds <= 0
  ) {
    return {
      refresh: async () => {},
      stop: () => {},
    };
  }

  let cancelled = false;
  let running = false;
  let timerId;

  const reportError = (error) => {
    if (onError) {
      onError(error);
    } else if (!cancelled) {
      console.error('Polling callback failed:', error);
    }
  };

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
      await callback();
    } catch (error) {
      reportError(error);
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

  return {
    refresh: run,
    stop: () => {
      cancelled = true;
      if (timerId !== undefined) globalThis.clearTimeout(timerId);
      if (pauseWhenHidden && typeof document !== 'undefined') {
        document.removeEventListener(
          'visibilitychange',
          handleVisibilityChange,
        );
      }
    },
  };
};

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

    const controller = createPollingController(
      () => callbackRef.current(),
      intervalMilliseconds,
      { immediate, pauseWhenHidden },
    );

    return controller.stop;
  }, [enabled, immediate, intervalMilliseconds, pauseWhenHidden, resetKey]);
};
