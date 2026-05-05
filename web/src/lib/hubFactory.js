import { rootUrl } from '../config';

const RECONNECT_DELAYS_MS = [
  0, 100, 250, 500, 1_000, 2_000, 3_000, 5_000, 5_000, 5_000, 5_000, 5_000,
];

const topicAliases = {
  application: new Set(['application', 'session']),
  logs: new Set(['logs']),
  search: new Set(['searches', 'search']),
  songid: new Set(['songid']),
  'listening-party': new Set(['listening-party']),
  transfers: new Set(['transfers', 'transfer']),
};

const eventAliases = {
  application: {
    'session.updated': 'state',
    'config.updated': 'options',
  },
  search: {
    'search.created': 'create',
    'search.started': 'create',
    'search.updated': 'update',
    'search.completed': 'update',
    'search.deleted': 'delete',
    'search.list': 'list',
  },
  transfers: {
    'transfer.started': 'activity',
    'transfer.progress': 'activity',
    'transfer.completed': 'activity',
    'transfer.failed': 'activity',
  },
  logs: {
    'log.buffer': 'buffer',
    'log.created': 'log',
  },
};

const eventFeedUrl = () => {
  const url = new URL(`${rootUrl || ''}/api/events/ws`, window.location.origin);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.toString();
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
    if (this.socket) {
      this.socket.close();
      this.socket = undefined;
    }
    return Promise.resolve();
  }

  connect() {
    return new Promise((resolve, reject) => {
      const socket = new WebSocket(eventFeedUrl());
      this.socket = socket;

      socket.onopen = () => {
        const wasReconnect = this.reconnectAttempt > 0;
        this.reconnectAttempt = 0;
        if (wasReconnect) {
          this.emitLifecycle(this.reconnectedHandlers);
        }
        resolve();
      };

      socket.onmessage = (message) => this.handleMessage(message.data);

      socket.onerror = () => {
        const error = new Error('WebSocket connection error');
        if (socket.readyState !== WebSocket.OPEN) {
          reject(error);
        }
      };

      socket.onclose = () => {
        if (this.socket === socket) {
          this.socket = undefined;
        }
        if (this.closedByClient) {
          return;
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

export const createHubConnection = ({ topic }) => new WebSocketHubConnection(topic);

export const createApplicationHubConnection = () =>
  createHubConnection({ topic: 'application' });

export const createLogsHubConnection = () => createHubConnection({ topic: 'logs' });

export const createSearchHubConnection = () =>
  createHubConnection({ topic: 'search' });

export const createSongIdHubConnection = () =>
  createHubConnection({ topic: 'songid' });

export const createListeningPartyHubConnection = () =>
  createHubConnection({ topic: 'listening-party' });

export const createTransfersHubConnection = () =>
  createHubConnection({ topic: 'transfers' });
