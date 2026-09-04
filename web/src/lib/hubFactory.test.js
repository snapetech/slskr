import { tokenPassthroughValue } from '../config';
import {
  createApplicationHubConnection,
  createMetricsHubConnection,
  createMessagesHubConnection,
  eventFeedProtocols,
  websocketAuthProtocolPrefix,
} from './hubFactory';
import { setToken } from './token';
import { vi } from 'vitest';

vi.mock('@microsoft/signalr', () => {
  class HubConnectionBuilder {
    withUrl() {
      return this;
    }

    withAutomaticReconnect() {
      return this;
    }

    withHubProtocol() {
      return this;
    }

    configureLogging() {
      return this;
    }

    build() {
      return {
        invoke: vi.fn(),
        on: vi.fn(),
        start: vi.fn(),
        state: 'Disconnected',
        stop: vi.fn().mockResolvedValue(undefined),
      };
    }
  }

  return {
    HubConnectionBuilder,
    JsonHubProtocol: class JsonHubProtocol {},
    LogLevel: { Warning: 3 },
  };
});

describe('event feed websocket auth', () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('sends browser-safe auth through a websocket subprotocol', () => {
    setToken(sessionStorage, 'route-token/with space');

    expect(eventFeedProtocols()).toEqual([
      `${websocketAuthProtocolPrefix}route-token%2Fwith%20space`,
    ]);
  });

  it('omits auth subprotocols for passthrough and missing tokens', () => {
    expect(eventFeedProtocols()).toEqual([]);

    setToken(sessionStorage, tokenPassthroughValue);

    expect(eventFeedProtocols()).toEqual([]);
  });

  it('exposes the upstream SignalR invocation and lifecycle surface for target hubs', async () => {
    const connection = createApplicationHubConnection();

    expect(connection.state).toBe('Disconnected');
    expect(connection.invoke).toEqual(expect.any(Function));
    expect(connection.on).toEqual(expect.any(Function));
    expect(connection.start).toEqual(expect.any(Function));
    expect(connection.stop).toEqual(expect.any(Function));

    await connection.stop();
  });

  it('exposes the frozen metrics hub connection', async () => {
    const connection = createMetricsHubConnection();

    expect(connection.invoke).toEqual(expect.any(Function));
    expect(connection.on).toEqual(expect.any(Function));

    await connection.stop();
  });
});

class MockEventFeedWebSocket {
  static OPEN = 1;
  static instances = [];

  readyState = 0;
  onopen = null;
  onmessage = null;
  onerror = null;
  onclose = null;

  constructor() {
    MockEventFeedWebSocket.instances.push(this);
  }

  close() {
    this.readyState = 3;
    this.onclose?.();
  }
}

describe('fallback event feed websocket lifecycle', () => {
  const originalWebSocket = globalThis.WebSocket;

  beforeEach(() => {
    vi.useFakeTimers();
    MockEventFeedWebSocket.instances = [];
    globalThis.WebSocket = MockEventFeedWebSocket;
  });

  afterEach(() => {
    vi.useRealTimers();
    globalThis.WebSocket = originalWebSocket;
  });

  it('rejects and closes a socket that never completes its handshake', async () => {
    const connection = createMessagesHubConnection();
    const started = connection.start();

    vi.advanceTimersByTime(15_000);

    await expect(started).rejects.toThrow('WebSocket connection timed out');
    expect(MockEventFeedWebSocket.instances).toHaveLength(1);
    expect(MockEventFeedWebSocket.instances[0].readyState).toBe(3);
    await connection.stop();
  });

  it('ignores callbacks from a socket replaced by a newer connection', async () => {
    const connection = createMessagesHubConnection();
    const first = connection.start();
    const firstSocket = MockEventFeedWebSocket.instances[0];
    await connection.stop();
    await expect(first).rejects.toThrow('WebSocket closed before opening');

    const second = connection.start();
    expect(MockEventFeedWebSocket.instances).toHaveLength(2);
    const secondSocket = MockEventFeedWebSocket.instances[1];
    firstSocket.onopen?.();
    expect(secondSocket.readyState).toBe(0);
    secondSocket.readyState = MockEventFeedWebSocket.OPEN;
    secondSocket.onopen?.();
    await second;
    await connection.stop();
  });
});
