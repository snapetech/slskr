import React from 'react';
import { cleanup, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { usePolling } from './usePolling';

const PollingProbe = ({ callback, interval = 100 }) => {
  usePolling(callback, interval);
  return null;
};

describe('usePolling', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('runs immediately and waits between completed refreshes', async () => {
    const callback = vi.fn();
    render(<PollingProbe callback={callback} />);

    expect(callback).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(99);
    expect(callback).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(callback).toHaveBeenCalledTimes(2);
  });

  it('does not overlap a slow refresh', async () => {
    let resolveRefresh;
    const callback = vi.fn(
      () =>
        new Promise((resolve) => {
          resolveRefresh = resolve;
        }),
    );
    render(<PollingProbe callback={callback} />);

    expect(callback).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1_000);
    expect(callback).toHaveBeenCalledTimes(1);

    resolveRefresh();
    await vi.advanceTimersByTimeAsync(99);
    expect(callback).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(callback).toHaveBeenCalledTimes(2);
  });

  it('pauses hidden refreshes and catches up when visible', async () => {
    const callback = vi.fn();
    render(<PollingProbe callback={callback} />);

    expect(callback).toHaveBeenCalledTimes(1);
    Object.defineProperty(document, 'visibilityState', { value: 'hidden' });
    await vi.advanceTimersByTimeAsync(500);
    expect(callback).toHaveBeenCalledTimes(1);

    Object.defineProperty(document, 'visibilityState', { value: 'visible' });
    document.dispatchEvent(new Event('visibilitychange'));
    await vi.advanceTimersByTimeAsync(0);
    expect(callback).toHaveBeenCalledTimes(2);
  });
});
