import { SlskrClient } from './client';
import { NetworkError } from './errors';

describe('SlskrClient request lifecycle', () => {
  it('validates and normalizes the REST base URL', () => {
    expect(() => new SlskrClient({ baseUrl: 'ftp://example.test', token: 'token' })).toThrow(
      'absolute HTTP or HTTPS'
    );
    expect(
      () => new SlskrClient({ baseUrl: 'https://user:pass@example.test', token: 'token' })
    ).toThrow('without credentials');

    const client = new SlskrClient({
      baseUrl: 'https://example.test/slskr/?debug=true#fragment',
      token: 'token',
    });
    expect((client as any).baseUrl).toBe('https://example.test/slskr');
  });
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

  it('uses daemon wire contracts and accepts daemon collection shapes', async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    global.fetch = jest.fn().mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      requests.push({ url, init });
      const parsed = new URL(url);

      if (parsed.pathname === '/api/searches') {
        return new Response('[{"id":"search-1"}]', { status: 200 });
      }
      if (parsed.pathname === '/api/messages' && init?.method === 'GET') {
        return new Response('{"entries":[{"id":"message-1"}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/messages/alice') {
        return new Response('{"entries":[{"id":"message-2"}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/messages' && init?.method === 'POST') {
        expect(JSON.parse(String(init.body))).toEqual({ username: 'alice', body: 'hello' });
        return new Response('{"id":"message-3"}', { status: 200 });
      }
      if (parsed.pathname === '/api/messages/7/ack') {
        return new Response(null, { status: 204 });
      }
      if (parsed.pathname === '/api/transfers') {
        if (parsed.searchParams.get('direction') !== '0') {
          throw new Error(`unexpected transfer query: ${parsed.search}`);
        }
        return new Response('{"entries":[{"id":"transfer-1"}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/rooms') {
        return new Response('{"entries":[{"name":"lounge"}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/rooms/lounge%20room/join') {
        return new Response(null, { status: 204 });
      }
      if (parsed.pathname === '/api/rooms/lounge%20room') {
        return new Response('{"name":"lounge room"}', { status: 200 });
      }
      if (parsed.pathname === '/api/events') {
        return new Response('[{"type":"message"}]', { status: 200 });
      }
      return new Response('{}', { status: 200 });
    });

    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 0,
    });

    await expect(client.listSearches()).resolves.toMatchObject([{ id: 'search-1', status: 'active' }]);
    await expect(client.listMessages()).resolves.toMatchObject([{ id: 'message-1' }]);
    await expect(client.getUserMessages('alice')).resolves.toMatchObject([{ id: 'message-2' }]);
    await expect(client.sendMessage({ recipient: 'alice', content: 'hello' })).resolves.toMatchObject({
      id: 'message-3',
    });
    await expect(client.acknowledgeMessage('7')).resolves.toBeUndefined();
    await expect(client.listTransfers({ direction: 'download' })).resolves.toMatchObject([
      { id: 'transfer-1' },
    ]);
    await expect(client.listRooms()).resolves.toEqual([{ name: 'lounge' }]);
    await expect(client.joinRoom('lounge room')).resolves.toBeUndefined();
    await expect(client.leaveRoom('lounge room')).resolves.toBeUndefined();
    await expect(client.getEvents()).resolves.toMatchObject([{ type: 'message', data: {} }]);

    expect(requests.find((request) => request.url.includes('/api/messages/7/ack'))?.init?.method).toBe('PUT');
    const transferRequest = requests.find((request) => request.url.includes('/api/transfers'));
    expect(transferRequest?.url).toContain('direction=0');
    expect(requests.some((request) => request.url.endsWith('/api/rooms/lounge%20room/join'))).toBe(true);
  });

  it('uses canonical session, browse, and MediaCore cache routes', async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    global.fetch = jest.fn().mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      requests.push({ url, init });
      const parsed = new URL(url);

      if (parsed.pathname === '/api/session' && init?.method === 'GET') {
        return new Response('{"state":"connected","username":"alice","privileges_seconds":120,"connected_at":1700000000}', { status: 200 });
      }
      if (parsed.pathname === '/api/server' && init?.method === 'PUT') {
        expect(JSON.parse(String(init.body))).toEqual({ username: 'alice', password: 'secret' });
        return new Response('{"accepted":true}', { status: 202 });
      }
      if (parsed.pathname === '/api/session/connect' || parsed.pathname === '/api/session/ping' || parsed.pathname === '/api/session/disconnect' || parsed.pathname === '/api/session/privileges/check') {
        return new Response('{"accepted":true}', { status: 202 });
      }
      if (parsed.pathname === '/api/users/alice/browse' && init?.method === 'GET') {
        return new Response('{"directories":[{"name":"Albums","files":[{"filename":"Albums/track.flac","size":42}]}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/users/alice/browse/request') {
        return new Response('{"username":"alice","status":"requested","requested_at":1700000000}', { status: 202 });
      }
      if (parsed.pathname === '/api/browse/requests') {
        return new Response('{"requests":[{"username":"alice","status":"ready","requested_at":1700000000}]}', { status: 200 });
      }
      if (parsed.pathname === '/api/users/alice/browse/cancel') {
        return new Response('{"entries":[]}', { status: 200 });
      }
      if (parsed.pathname === '/api/mediacore/retrieve/stats') {
        return new Response('{"totalRetrievals":4,"cacheHits":3,"cacheMisses":1,"cacheHitRatio":0.75,"expiredEntriesCleaned":2}', { status: 200 });
      }
      if (parsed.pathname === '/api/mediacore/retrieve/cache/clear') {
        expect(JSON.parse(String(init?.body))).toEqual({ keys: ['content:audio:track:1'] });
        return new Response('{"success":true}', { status: 200 });
      }
      return new Response('{}', { status: 200 });
    });

    const client = new SlskrClient({
      baseUrl: 'http://localhost:8080',
      token: 'test-token',
      retries: 0,
    });

    await expect(client.getSessions()).resolves.toMatchObject([
      { id: 'server', type: 'server', status: 'connected' },
    ]);
    await expect(client.createSession('server', { username: 'alice', password: 'secret' })).resolves.toMatchObject({
      id: 'server',
      status: 'connected',
    });
    await expect(client.pingSession('server')).resolves.toMatchObject({ status: 'accepted' });
    await expect(client.disconnectSession('server')).resolves.toBeUndefined();
    await expect(client.getSessionPrivileges('server')).resolves.toEqual({
      user_id: 'alice',
      privileges: ['privileged'],
    });
    await expect(client.browseUser('alice')).resolves.toEqual({
      entries: [{ filename: 'Albums/track.flac', size: 42 }],
    });
    await expect(client.requestBrowse('alice')).resolves.toMatchObject({
      id: 'alice',
      from: 'alice',
      status: 'pending',
    });
    await expect(client.getBrowseRequests()).resolves.toMatchObject([
      { id: 'alice', status: 'accepted' },
    ]);
    await expect(client.respondToBrowseRequest('alice', 'reject')).resolves.toEqual({ entries: [] });
    await expect(client.getCacheStats()).resolves.toEqual({
      hits: 3,
      misses: 1,
      evictions: 2,
      total_requests: 4,
      hit_rate: 0.75,
    });
    await expect(client.invalidateCache(['content:audio:track:1'])).resolves.toBeUndefined();

    expect(requests.some((request) => request.url.endsWith('/api/sessions'))).toBe(false);
    expect(requests.some((request) => request.url.endsWith('/api/cache/stats'))).toBe(false);
    expect(requests.some((request) => request.url.endsWith('/api/cache/invalidate'))).toBe(false);
  });
});
