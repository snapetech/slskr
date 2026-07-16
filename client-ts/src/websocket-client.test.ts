import { WebSocketClient } from './websocket-client';

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
    this.onclose?.();
  }

  send(data: string): void {
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
    client.unsubscribe('transfer.completed', 'search.completed', 'search.completed');

    const frames = MockWebSocket.instances[0].sent.map((frame) => JSON.parse(frame));
    expect(frames).toEqual([
      { type: 'subscribe', data: { topics: ['search.completed'] } },
      { type: 'unsubscribe', data: { topics: ['search.completed'] } },
    ]);
  });
});
