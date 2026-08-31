import { tokenPassthroughValue } from '../config';
import {
  createApplicationHubConnection,
  createMetricsHubConnection,
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
