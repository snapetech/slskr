import { fetchWithoutRedirects } from './http';

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
