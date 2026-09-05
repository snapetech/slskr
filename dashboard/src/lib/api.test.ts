import { describe, expect, it, vi } from 'vitest';
import { isAbortError, readResponseText, requestJson } from './api';

describe('requestJson', () => {
  it('rejects redirects even when a caller requests follow behavior', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{"ok":true}', {
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    await expect(
      requestJson<{ ok: boolean }>('/api/config', 'secret', { redirect: 'follow' }),
    ).resolves.toEqual({ ok: true });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/config',
      expect.objectContaining({
        redirect: 'error',
        headers: expect.any(Headers),
      }),
    );
    const [, init] = fetchMock.mock.calls[0];
    expect(new Headers(init?.headers).get('Authorization')).toBe('Bearer secret');
  });
});

describe('readResponseText', () => {
  it('rejects a response that declares more bytes than the limit', async () => {
    await expect(
      readResponseText(
        new Response('ok', { headers: { 'Content-Length': '4' } }),
        3,
      ),
    ).rejects.toThrow('exceeds 3 bytes');
  });

  it('cancels a streaming response when it crosses the limit', async () => {
    const cancel = vi.fn().mockResolvedValue(undefined);
    const reader = {
      read: vi
        .fn()
        .mockResolvedValueOnce({ done: false, value: new Uint8Array([1, 2, 3, 4]) }),
      cancel,
    };
    const response = {
      body: { getReader: () => reader },
      headers: new Headers(),
    } as unknown as Response;

    await expect(readResponseText(response, 3)).rejects.toThrow('exceeds 3 bytes');
    expect(cancel).toHaveBeenCalledOnce();
  });
});

describe('isAbortError', () => {
  it('recognizes browser and fetch-style abort errors', () => {
    expect(isAbortError(new DOMException('aborted', 'AbortError'))).toBe(true);
    expect(isAbortError(Object.assign(new Error('aborted'), { name: 'AbortError' }))).toBe(true);
  });

  it('does not classify ordinary errors as aborts', () => {
    expect(isAbortError(new Error('failed'))).toBe(false);
    expect(isAbortError({ name: 'NetworkError' })).toBe(false);
    expect(isAbortError(null)).toBe(false);
  });
});
