import { MAX_WEBSOCKET_MESSAGE_BYTES, WebSocketClient } from './websocket-client';

class MockWebSocket {
  static readonly OPEN = 1;
  static instances: MockWebSocket[] = [];

  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  sent: string[] = [];
  url: string;
  sendError: Error | null = null;
  deferClose = false;

  constructor(url: string, _protocols?: string | string[]) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  open(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.();
  }

  close(): void {
    this.readyState = 3;
    if (!this.deferClose) {
      this.onclose?.();
    }
  }

  emitClose(): void {
    this.onclose?.();
  }

  error(): void {
    this.onerror?.();
  }

  send(data: string): void {
    if (this.sendError) throw this.sendError;
    this.sent.push(data);
  }
}

describe('WebSocketClient reconnect lifecycle', () => {
  const originalWebSocket = global.WebSocket;

  beforeEach(() => {
    jest.useFakeTimers();
    MockWebSocket.instances = [];
    global.WebSocket = MockWebSocket as unknown as typeof WebSocket;
  });

  afterEach(() => {
    jest.useRealTimers();
    global.WebSocket = originalWebSocket;
  });

  it('does not reconnect after an intentional disconnect', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    client.disconnect();
    jest.runOnlyPendingTimers();

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(client.isConnected()).toBe(false);
  });

  it('does not send text frames as a WebSocket control keepalive', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    jest.advanceTimersByTime(30_000);

    expect(MockWebSocket.instances[0].sent).toEqual([]);
    client.disconnect();
  });

  it('still reconnects after an unexpected close', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    MockWebSocket.instances[0].close();
    jest.advanceTimersByTime(1000);

    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it('rejects when the socket closes before opening', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();

    MockWebSocket.instances[0].close();

    await expect(connected).rejects.toThrow('WebSocket closed before opening');
  });

  it('rejects and closes a socket that never completes its handshake', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();

    jest.advanceTimersByTime(15_000);

    await expect(connected).rejects.toThrow('WebSocket connection timed out');
    expect(client.isConnected()).toBe(false);
    client.disconnect();
  });

  it('cleans up a timed-out handshake so callers can retry immediately', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const firstConnection = client.connect();

    jest.advanceTimersByTime(15_000);

    await expect(firstConnection).rejects.toThrow('WebSocket connection timed out');
    const secondConnection = client.connect();
    expect(MockWebSocket.instances).toHaveLength(2);
    MockWebSocket.instances[1].open();
    await secondConnection;
    client.disconnect();
  });

  it('cleans up a handshake error so callers can retry immediately', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const firstConnection = client.connect();

    MockWebSocket.instances[0].error();
    await expect(firstConnection).rejects.toThrow('WebSocket connection error');

    const secondConnection = client.connect();
    expect(MockWebSocket.instances).toHaveLength(2);
    MockWebSocket.instances[1].open();
    await secondConnection;
    client.disconnect();
  });

  it('rejects concurrent connection attempts without replacing the active socket', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const firstConnection = client.connect();

    await expect(client.connect()).rejects.toThrow('already in progress');
    expect(MockWebSocket.instances).toHaveLength(1);

    MockWebSocket.instances[0].open();
    await firstConnection;
    await expect(client.connect()).rejects.toThrow('already connected');
    client.disconnect();
  });

  it('settles an in-flight connection when intentionally disconnected', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();

    client.disconnect();

    await expect(connected).rejects.toThrow('closed before opening');
    jest.runOnlyPendingTimers();
    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it('allows immediate reconnect before the old close event arrives', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    const oldSocket = MockWebSocket.instances[0];
    oldSocket.open();
    await connected;

    oldSocket.deferClose = true;
    client.disconnect();

    const reconnected = client.connect();
    expect(MockWebSocket.instances).toHaveLength(2);
    MockWebSocket.instances[1].open();
    await reconnected;

    oldSocket.emitClose();
    expect(client.isConnected()).toBe(true);
    client.disconnect();
  });

  it('validates and normalizes the WebSocket endpoint URL', async () => {
    expect(() => new WebSocketClient('ftp://example.test', 'token')).toThrow(
      'absolute HTTP or HTTPS'
    );
    expect(() => new WebSocketClient('https://user:pass@example.test', 'token')).toThrow(
      'without credentials'
    );

    const client = new WebSocketClient(
      'https://example.test/slskr/?debug=true#fragment',
      'token'
    );
    const connected = client.connect();
    expect(MockWebSocket.instances[0].url).toBe('wss://example.test/slskr/api/events/ws');
    MockWebSocket.instances[0].open();
    await connected;
    client.disconnect();
  });

  it('restores subscriptions after reconnecting', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    client.subscribe('search.completed', 'transfer.completed');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    expect(JSON.parse(MockWebSocket.instances[0].sent[0])).toEqual({
      type: 'subscribe',
      data: { topics: ['search.completed', 'transfer.completed'] },
    });
    MockWebSocket.instances[0].close();
    jest.advanceTimersByTime(1000);
    MockWebSocket.instances[1].open();

    expect(JSON.parse(MockWebSocket.instances[1].sent[0])).toEqual({
      type: 'subscribe',
      data: { topics: ['search.completed', 'transfer.completed'] },
    });
  });

  it('sends only actual unsubscribe transitions', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    client.subscribe('search.completed');
    client.subscribe('transfer.completed', 'transfer.completed');
    client.unsubscribe('transfer.completed', 'search.completed', 'search.completed');

    const frames = MockWebSocket.instances[0].sent.map((frame) => JSON.parse(frame));
    expect(frames).toEqual([
      { type: 'subscribe', data: { topics: ['search.completed'] } },
      { type: 'subscribe', data: { topics: ['transfer.completed'] } },
      {
        type: 'unsubscribe',
        data: { topics: ['transfer.completed', 'search.completed'] },
      },
    ]);
  });

  it('rolls back subscription state when a write throws', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    MockWebSocket.instances[0].sendError = new Error('write failed');
    expect(() => client.subscribe('search.completed')).toThrow('write failed');
    expect(client.getSubscribedTopics()).toEqual([]);

    MockWebSocket.instances[0].sendError = null;
    client.subscribe('transfer.completed');
    MockWebSocket.instances[0].sendError = new Error('write failed');
    expect(() => client.unsubscribe('transfer.completed')).toThrow('write failed');
    expect(client.getSubscribedTopics()).toEqual(['transfer.completed']);
    client.disconnect();
  });

  it('rejects connect when restoring subscriptions throws', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    client.subscribe('search.completed');
    const connected = client.connect();
    MockWebSocket.instances[0].sendError = new Error('restore failed');

    MockWebSocket.instances[0].open();

    await expect(connected).rejects.toThrow('restore failed');
    expect(client.isConnected()).toBe(false);
  });

  it('rejects oversized and non-object event messages before dispatch', async () => {
    const client = new WebSocketClient('http://localhost:8080', 'token');
    const connected = client.connect();
    MockWebSocket.instances[0].open();
    await connected;

    const received = jest.fn();
    const errors = jest.fn();
    client.on('search.started', received);
    client.onError(errors);
    MockWebSocket.instances[0].onmessage?.({ data: 'null' });
    MockWebSocket.instances[0].onmessage?.({
      data: JSON.stringify({
        data: { id: 'search-1' },
        id: 'event-1',
        timestamp: '2026-09-05T00:00:00Z',
        type: 'search.started',
      }),
    });
    MockWebSocket.instances[0].onmessage?.({
      data: 'x'.repeat(MAX_WEBSOCKET_MESSAGE_BYTES + 1),
    });

    expect(received).toHaveBeenCalledTimes(1);
    expect(errors).toHaveBeenCalledTimes(1);
    expect(errors.mock.calls[0][0]).toHaveProperty(
      'message',
      `WebSocket message exceeds ${MAX_WEBSOCKET_MESSAGE_BYTES} bytes`,
    );
    client.disconnect();
  });
});
