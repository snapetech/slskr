import { hubBaseUrl, rootUrl } from '../config';
import { getToken, isPassthroughEnabled } from './token';
import {
  HubConnectionBuilder,
  JsonHubProtocol,
  LogLevel,
} from '@microsoft/signalr';

const RECONNECT_DELAYS_MS = [
  0, 100, 250, 500, 1_000, 2_000, 3_000, 5_000, 5_000, 5_000, 5_000, 5_000,
];
export const WEBSOCKET_CONNECT_TIMEOUT_MS = 15_000;

const SIGNALR_HUB_TOPICS = new Set([
  'application',
  'logs',
  'search',
  'metrics',
  'songid',
  'listening-party',
  'transfers',
]);

const topicAliases = {
  application: new Set(['application', 'session']),
  logs: new Set(['logs']),
  messages: new Set(['messages']),
  rooms: new Set(['rooms']),
  search: new Set(['searches', 'search']),
  songid: new Set(['songid']),
  'listening-party': new Set(['listening-party']),
  transfers: new Set(['transfers', 'transfer']),
};

const eventAliases = {
  application: {
    STATE: 'state',
    OPTIONS: 'options',
    'session.updated': 'state',
    'config.updated': 'options',
  },
  search: {
    LIST: 'list',
    CREATE: 'create',
    UPDATE: 'update',
    DELETE: 'delete',
    'search.created': 'create',
    'search.started': 'create',
    'search.updated': 'update',
    'search.completed': 'update',
    'search.deleted': 'delete',
    'search.list': 'list',
  },
  transfers: {
    ACTIVITY: 'activity',
    PROGRESS: 'activity',
    REMOVED: 'activity',
    'transfer.started': 'activity',
    'transfer.progress': 'activity',
    'transfer.completed': 'activity',
    'transfer.failed': 'activity',
  },
  logs: {
    BUFFER: 'buffer',
    LOG: 'log',
    'log.buffer': 'buffer',
    'log.created': 'log',
  },
  messages: {
    'conversation.deleted': 'changed',
    'message.acked': 'changed',
    'message.received': 'changed',
    'message.sent': 'changed',
  },
  rooms: {
    'room.joined': 'changed',
    'room.left': 'changed',
    'room.list.updated': 'changed',
    'room.message': 'changed',
    'room.updated': 'changed',
    'room.users.updated': 'changed',
  },
};

const eventFeedUrl = () => {
  const url = new URL(`${rootUrl || ''}/api/events/ws`, window.location.origin);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.toString();
};

export const websocketAuthProtocolPrefix = 'slskr.api-token.';

export const eventFeedProtocols = () => {
  const token = getToken()?.trim();
  if (!token || isPassthroughEnabled()) {
    return [];
  }
  return [`${websocketAuthProtocolPrefix}${encodeURIComponent(token)}`];
};

class WebSocketHubConnection {
  constructor(topic) {
    this.topic = topic;
    this.handlers = new Map();
    this.closeHandlers = [];
    this.reconnectingHandlers = [];
    this.reconnectedHandlers = [];
    this.reconnectAttempt = 0;
    this.closedByClient = false;
    this.socket = undefined;
    this.connectionTimer = undefined;
    this.pendingConnection = undefined;
  }

  on(eventName, handler) {
    if (!this.handlers.has(eventName)) {
      this.handlers.set(eventName, new Set());
    }
    this.handlers.get(eventName).add(handler);
  }

  onclose(handler) {
    this.closeHandlers.push(handler);
  }

  onreconnecting(handler) {
    this.reconnectingHandlers.push(handler);
  }

  onreconnected(handler) {
    this.reconnectedHandlers.push(handler);
  }

  start() {
    this.closedByClient = false;
    return this.connect();
  }

  stop() {
    this.closedByClient = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }
    if (this.connectionTimer !== undefined) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = undefined;
    }
    if (this.pendingConnection) {
      this.pendingConnection.reject(new Error('WebSocket closed before opening'));
    }
    if (this.socket) {
      this.socket.close();
      this.socket = undefined;
    }
    return Promise.resolve();
  }

  connect() {
    if (this.socket) {
      return Promise.reject(
        new Error(
          this.socket.readyState === WebSocket.OPEN
            ? 'WebSocket is already connected'
            : 'WebSocket connection already in progress',
        ),
      );
    }

    return new Promise((resolve, reject) => {
      let settled = false;
      let pendingConnection;
      const clearConnectionTimer = () => {
        if (this.connectionTimer !== undefined) {
          clearTimeout(this.connectionTimer);
          this.connectionTimer = undefined;
        }
      };
      const settle = (callback) => {
        if (settled) {
          return false;
        }
        settled = true;
        clearConnectionTimer();
        if (this.pendingConnection === pendingConnection) {
          this.pendingConnection = undefined;
        }
        callback();
        return true;
      };

      let socket;
      try {
        socket = new WebSocket(eventFeedUrl(), eventFeedProtocols());
        this.socket = socket;
      } catch (error) {
        settle(() => reject(error));
        return;
      }

      pendingConnection = {
        reject: (error) => settle(() => reject(error)),
        socket,
      };
      this.pendingConnection = pendingConnection;
      this.connectionTimer = setTimeout(() => {
        if (this.socket !== socket || settled) {
          return;
        }
        const error = new Error(
          `WebSocket connection timed out after ${WEBSOCKET_CONNECT_TIMEOUT_MS}ms`,
        );
        settle(() => reject(error));
        socket.close();
      }, WEBSOCKET_CONNECT_TIMEOUT_MS);

      socket.onopen = () => {
        if (this.socket !== socket || settled) {
          return;
        }
        clearConnectionTimer();
        const wasReconnect = this.reconnectAttempt > 0;
        this.reconnectAttempt = 0;
        if (wasReconnect) {
          this.emitLifecycle(this.reconnectedHandlers);
        }
        settle(resolve);
      };

      socket.onmessage = (message) => {
        if (this.socket !== socket) {
          return;
        }
        this.handleMessage(message.data);
      };

      socket.onerror = () => {
        if (this.socket !== socket) {
          return;
        }
        const error = new Error('WebSocket connection error');
        if (!settled && socket.readyState !== WebSocket.OPEN) {
          settle(() => reject(error));
          socket.close();
        }
      };

      socket.onclose = () => {
        if (this.socket !== socket) {
          return;
        }
        this.socket = undefined;
        clearConnectionTimer();
        if (this.closedByClient) {
          if (!settled) {
            settle(() => reject(new Error('WebSocket closed before opening')));
          }
          return;
        }
        if (!settled) {
          settle(() => reject(new Error('WebSocket closed before opening')));
        }
        const error = new Error('WebSocket disconnected');
        this.emitLifecycle(this.reconnectingHandlers, error);
        this.scheduleReconnect(error);
      };
    });
  }

  scheduleReconnect(error) {
    const delay =
      RECONNECT_DELAYS_MS[
        Math.min(this.reconnectAttempt, RECONNECT_DELAYS_MS.length - 1)
      ];
    this.reconnectAttempt += 1;
    this.reconnectTimer = setTimeout(() => {
      this.connect().catch((connectError) => {
        if (this.reconnectAttempt >= RECONNECT_DELAYS_MS.length) {
          this.emitLifecycle(this.closeHandlers, connectError || error);
          return;
        }
        this.scheduleReconnect(connectError || error);
      });
    }, delay);
  }

  handleMessage(data) {
    let message;
    try {
      message = JSON.parse(data);
    } catch {
      return;
    }

    if (!this.acceptsTopic(message.topic)) {
      return;
    }

    const eventName =
      eventAliases[this.topic]?.[message.type] ?? message.type ?? 'event';
    this.emit(eventName, message.data ?? message.event ?? message);
  }

  acceptsTopic(topic) {
    const accepted = topicAliases[this.topic] ?? new Set([this.topic]);
    return accepted.has(topic);
  }

  emit(eventName, payload) {
    const handlers = this.handlers.get(eventName);
    if (!handlers) {
      return;
    }
    for (const handler of handlers) {
      handler(payload);
    }
  }

  emitLifecycle(handlers, arg) {
    for (const handler of handlers) {
      handler(arg);
    }
  }
}

const createSignalRHubConnection = (topic) =>
  new HubConnectionBuilder()
    .withUrl(`${hubBaseUrl}/${topic}`, {
      accessTokenFactory: isPassthroughEnabled() ? undefined : getToken,
      withCredentials: true,
    })
    .withAutomaticReconnect(RECONNECT_DELAYS_MS)
    .withHubProtocol(new JsonHubProtocol())
    .configureLogging(LogLevel.Warning)
    .build();

export const createHubConnection = ({ topic }) =>
  SIGNALR_HUB_TOPICS.has(topic)
    ? createSignalRHubConnection(topic)
    : new WebSocketHubConnection(topic);

export const createApplicationHubConnection = () =>
  createHubConnection({ topic: 'application' });

export const createLogsHubConnection = () => createHubConnection({ topic: 'logs' });

export const createMessagesHubConnection = () =>
  createHubConnection({ topic: 'messages' });

export const createRoomsHubConnection = () => createHubConnection({ topic: 'rooms' });

export const createSearchHubConnection = () =>
  createHubConnection({ topic: 'search' });

export const createMetricsHubConnection = () =>
  createHubConnection({ topic: 'metrics' });

export const createSongIdHubConnection = () =>
  createHubConnection({ topic: 'songid' });

export const createListeningPartyHubConnection = () =>
  createHubConnection({ topic: 'listening-party' });

export const createTransfersHubConnection = () =>
  createHubConnection({ topic: 'transfers' });
