import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useFetch } from './useFetch';

describe('useFetch', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('does not restart polling when the options object is recreated', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(JSON.stringify({ status: 'ok' }), {
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const { result, rerender } = renderHook(() =>
      useFetch<{ status: string }>('/api/health', { interval: 60_000 }),
    );

    await waitFor(() => expect(result.current.data).toEqual({ status: 'ok' }));
    rerender();

    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('rejects redirects and accepts an empty successful response', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(null, { status: 204 }),
    );

    const { result } = renderHook(() => useFetch<undefined>('/api/health'));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBeNull();
    expect(result.current.data).toBeUndefined();
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/health',
      expect.objectContaining({ redirect: 'error' }),
    );
  });

  it('does not let an older request clear loading for a newer request', async () => {
    let resolveFirst: ((response: Response) => void) | undefined;
    const firstResponse = new Promise<Response>((resolve) => {
      resolveFirst = resolve;
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch')
      .mockReturnValueOnce(firstResponse)
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ value: 2 }), {
          headers: { 'Content-Type': 'application/json' },
        }),
      );

    const { result } = renderHook(() => useFetch<{ value: number }>('/api/value'));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.data).toEqual({ value: 2 });
    expect(result.current.loading).toBe(false);
    await act(async () => {
      resolveFirst?.(
        new Response(JSON.stringify({ value: 1 }), {
          headers: { 'Content-Type': 'application/json' },
        }),
      );
      await firstResponse;
    });
    expect(result.current.data).toEqual({ value: 2 });
  });

  it('clears stale state and stops loading when the URL is disabled', async () => {
    let resolveRequest: ((response: Response) => void) | undefined;
    const pendingResponse = new Promise<Response>((resolve) => {
      resolveRequest = resolve;
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockReturnValue(pendingResponse);

    const { result, rerender } = renderHook(
      ({ url }: { url: string | null }) => useFetch<{ value: number }>(url),
      { initialProps: { url: '/api/value' as string | null } },
    );

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(result.current.loading).toBe(true);

    rerender({ url: null });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveRequest?.(
        new Response(JSON.stringify({ value: 1 }), {
          headers: { 'Content-Type': 'application/json' },
        }),
      );
      await pendingResponse;
    });

    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(false);
  });
});
