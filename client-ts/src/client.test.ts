import { SlskrClient } from './client';
import { NetworkError } from './errors';

describe('SlskrClient request lifecycle', () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it('clears request timeout timers when fetch rejects', async () => {
    const clearTimeoutSpy = jest.spyOn(global, 'clearTimeout');
    global.fetch = jest.fn().mockRejectedValue(new Error('network down'));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      timeout: 1000,
      retries: 0,
    });

    await expect(client.health()).rejects.toBeInstanceOf(NetworkError);

    expect(clearTimeoutSpy).toHaveBeenCalledTimes(1);
  });

  it('honors explicit zero lifecycle configuration', () => {
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      timeout: 0,
      retries: 0,
      retryDelay: 0,
      debug: false,
    });

    expect((client as any).timeout).toBe(0);
    expect((client as any).retries).toBe(0);
    expect((client as any).retryDelay).toBe(0);
    expect((client as any).debug).toBe(false);
  });
});
