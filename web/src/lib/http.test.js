import { fetchWithoutRedirects, readJsonResponse } from './http';

describe('fetchWithoutRedirects', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('overrides caller redirect behavior', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{}'),
    );

    await fetchWithoutRedirects('/api/health', { redirect: 'follow' });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/health',
      expect.objectContaining({ redirect: 'error' }),
    );
  });
});

describe('readJsonResponse', () => {
  it('parses complete JSON from a streaming response', async () => {
    await expect(readJsonResponse(new Response('{"ok":true}'))).resolves.toEqual({
      ok: true,
    });
  });

  it('returns undefined for empty text and empty streams', async () => {
    await expect(
      readJsonResponse({
        body: null,
        headers: new Headers(),
        text: async () => '  ',
      }),
    ).resolves.toBeUndefined();
    await expect(readJsonResponse(new Response('  '))).resolves.toBeUndefined();
  });

  it('parses JSON from a response without a readable stream', async () => {
    await expect(
      readJsonResponse({
        body: null,
        headers: new Headers(),
        text: async () => '{"ok":true}',
      }),
    ).resolves.toEqual({ ok: true });
  });

  it('rejects oversized text fallback by its encoded byte length', async () => {
    await expect(
      readJsonResponse(
        {
          body: null,
          headers: new Headers(),
          text: async () => 'éé',
        },
        3,
      ),
    ).rejects.toThrow('exceeds 3 bytes');
  });

  it('rejects a response that declares more bytes than the limit', async () => {
    await expect(
      readJsonResponse(
        new Response('{"ok":true}', {
          headers: { 'Content-Length': '17' },
        }),
        16,
      ),
    ).rejects.toThrow('exceeds 16 bytes');
  });

  it('cancels the body when the declared length exceeds the limit', async () => {
    const cancel = vi.fn().mockResolvedValue(undefined);

    await expect(
      readJsonResponse(
        {
          body: { cancel },
          headers: new Headers({ 'Content-Length': '4' }),
        },
        3,
      ),
    ).rejects.toThrow('exceeds 3 bytes');
    expect(cancel).toHaveBeenCalledOnce();
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
    };

    await expect(readJsonResponse(response, 3)).rejects.toThrow('exceeds 3 bytes');
    expect(cancel).toHaveBeenCalledOnce();
  });
});
