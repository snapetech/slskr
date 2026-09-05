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
