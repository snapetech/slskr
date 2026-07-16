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

  it('accepts successful no-content mutations without retrying JSON parsing', async () => {
    global.fetch = jest.fn().mockResolvedValue(new Response(null, { status: 204 }));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 3,
    });

    await expect(client.disconnectSession('session')).resolves.toBeUndefined();
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it('rejects oversized declared responses without retrying', async () => {
    global.fetch = jest.fn().mockResolvedValue(new Response(null, {
      status: 200,
      headers: { 'content-length': String(8 * 1024 * 1024 + 1) },
    }));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 3,
    });

    await expect(client.health()).rejects.toBeInstanceOf(NetworkError);
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it('bounds streamed responses without a content length', async () => {
    const chunk = new Uint8Array(8 * 1024 * 1024 + 1);
    global.fetch = jest.fn().mockResolvedValue(new Response(new ReadableStream({
      start(controller) {
        controller.enqueue(chunk);
        controller.close();
      },
    }), { status: 200 }));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 0,
    });

    await expect(client.health()).rejects.toBeInstanceOf(NetworkError);
  });

  it('does not replay mutations after a transport failure', async () => {
    global.fetch = jest.fn().mockRejectedValue(new Error('response lost'));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 3,
      retryDelay: 0,
    });

    await expect(client.createSearch({ query: 'rare' })).rejects.toBeInstanceOf(NetworkError);
    expect(global.fetch).toHaveBeenCalledTimes(1);
  });

  it('retains retries for idempotent reads', async () => {
    global.fetch = jest
      .fn()
      .mockRejectedValueOnce(new Error('network down'))
      .mockResolvedValueOnce(new Response('{"status":"ok"}', { status: 200 }));
    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 1,
      retryDelay: 0,
    });

    await expect(client.health()).resolves.toMatchObject({ status: 'ok' });
    expect(global.fetch).toHaveBeenCalledTimes(2);
  });
});
