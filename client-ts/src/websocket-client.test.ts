import { WebSocketClient } from './websocket-client';

class MockWebSocket {
  static readonly OPEN = 1;
  static instances: MockWebSocket[] = [];

  readyState = 0;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;

  constructor(_url: string, _protocols?: string | string[]) {
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

  send(_data: string): void {}
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
});
