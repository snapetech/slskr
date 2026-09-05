/**
 * WebSocket client for real-time event streaming
 */

import { Event, EventType, WebSocketMessage } from './types';

export const websocketAuthProtocolPrefix = 'slskr.api-token.';
export const WEBSOCKET_CONNECT_TIMEOUT_MS = 15_000;
export const MAX_WEBSOCKET_MESSAGE_BYTES = 64 * 1024;

export function websocketAuthProtocols(token: string): string[] {
  const normalized = token.trim();
  return normalized ? [`${websocketAuthProtocolPrefix}${encodeURIComponent(normalized)}`] : [];
}

export type EventListener = (event: Event) => void;
export type ConnectionListener = (connected: boolean) => void;
export type ErrorListener = (error: Error) => void;

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private connectionTimer: ReturnType<typeof setTimeout> | null = null;
  private pendingConnectReject: ((error: Error) => void) | null = null;
  private intentionallyDisconnected = false;
  private subscribedTopics: Set<EventType> = new Set();

  private listeners: Map<EventType, Set<EventListener>> = new Map();
  private connectionListeners: Set<ConnectionListener> = new Set();
  private errorListeners: Set<ErrorListener> = new Set();

  constructor(baseUrl: string, token: string) {
    const parsedUrl = new URL(baseUrl);
    if (!['http:', 'https:'].includes(parsedUrl.protocol) || parsedUrl.username || parsedUrl.password) {
      throw new Error('baseUrl must be an absolute HTTP or HTTPS URL without credentials');
    }
    parsedUrl.protocol = parsedUrl.protocol === 'https:' ? 'wss:' : 'ws:';
    parsedUrl.pathname = `${parsedUrl.pathname.replace(/\/+$/, '')}/api/events/ws`;
    parsedUrl.search = '';
    parsedUrl.hash = '';
    this.url = parsedUrl.toString();
    this.token = token;
  }

  /**
   * Connect to WebSocket
   */
  async connect(): Promise<void> {
    if (this.ws !== null) {
      throw new Error(
        this.ws.readyState === WebSocket.OPEN
          ? 'WebSocket is already connected'
          : 'WebSocket connection already in progress'
      );
    }
    this.intentionallyDisconnected = false;
    this.clearReconnectTimer();
    return new Promise((resolve, reject) => {
      let settled = false;
      const clearConnectionTimer = () => this.clearConnectionTimer();
      const settle = (callback: () => void) => {
        if (settled) return false;
        settled = true;
        clearConnectionTimer();
        this.pendingConnectReject = null;
        callback();
        return true;
      };

      try {
        const socket = new WebSocket(this.url, websocketAuthProtocols(this.token));
        this.ws = socket;
        this.pendingConnectReject = (error) => settle(() => reject(error));
        this.connectionTimer = setTimeout(() => {
          if (this.ws !== socket || settled) return;
          const error = new Error(
            `WebSocket connection timed out after ${WEBSOCKET_CONNECT_TIMEOUT_MS}ms`
          );
          this.ws = null;
          settle(() => reject(error));
          this.attemptReconnect();
          this.notifyErrorListeners(error);
          socket.close();
        }, WEBSOCKET_CONNECT_TIMEOUT_MS);

        socket.onopen = () => {
          if (this.ws !== socket) return;
          if (settled) return;
          clearConnectionTimer();
          try {
            this.sendSubscription('subscribe', Array.from(this.subscribedTopics));
            this.reconnectAttempts = 0;
            this.notifyConnectionListeners(true);
            settle(resolve);
          } catch (error) {
            const connectionError = error instanceof Error ? error : new Error(String(error));
            this.ws = null;
            settle(() => reject(connectionError));
            this.attemptReconnect();
            this.notifyErrorListeners(connectionError);
            socket.close();
          }
        };

        socket.onmessage = (event) => {
          if (this.ws !== socket) return;
          this.handleMessage(event.data);
        };

        socket.onerror = () => {
          if (this.ws !== socket) return;
          const error = new Error('WebSocket error');
          const handshakeFailed = !settled && socket.readyState !== WebSocket.OPEN;
          if (handshakeFailed) {
            this.ws = null;
            settle(() => reject(new Error('WebSocket connection error')));
            this.attemptReconnect();
          }
          this.notifyErrorListeners(error);
          if (handshakeFailed) {
            socket.close();
          }
        };

        socket.onclose = () => {
          if (this.ws !== socket) return;
          this.ws = null;
          clearConnectionTimer();
          this.notifyConnectionListeners(false);
          if (!settled) {
            settle(() => reject(new Error('WebSocket closed before opening')));
          }
          if (!this.intentionallyDisconnected) {
            this.attemptReconnect();
          }
        };
      } catch (error) {
        settle(() => reject(error instanceof Error ? error : new Error(String(error))));
      }
    });
  }

  /**
   * Disconnect from WebSocket
   */
  disconnect(): void {
    this.intentionallyDisconnected = true;
    this.clearReconnectTimer();
    this.clearConnectionTimer();
    this.pendingConnectReject?.(new Error('WebSocket closed before opening'));
    const socket = this.ws;
    this.ws = null;
    if (socket) {
      // In browsers, close events are delivered asynchronously. Clear the
      // active socket before closing it so callers can reconnect immediately;
      // the old socket's callbacks are ignored by their identity checks.
      this.notifyConnectionListeners(false);
      socket.close();
    }
  }

  /**
   * Subscribe to event types
   */
  subscribe(...topics: EventType[]): void {
    const newTopics = topics.filter(
      (topic, index) => topics.indexOf(topic) === index && !this.subscribedTopics.has(topic)
    );
    if (newTopics.length === 0) return;

    newTopics.forEach((t) => this.subscribedTopics.add(t));

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      try {
        this.sendSubscription('subscribe', newTopics);
      } catch (error) {
        newTopics.forEach((topic) => this.subscribedTopics.delete(topic));
        throw error;
      }
    }
  }

  /**
   * Unsubscribe from event types
   */
  unsubscribe(...topics: EventType[]): void {
    const removedTopics = topics.filter((topic, index) =>
      topics.indexOf(topic) === index && this.subscribedTopics.delete(topic)
    );
    if (removedTopics.length === 0) return;

    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      try {
        this.sendSubscription('unsubscribe', removedTopics);
      } catch (error) {
        removedTopics.forEach((topic) => this.subscribedTopics.add(topic));
        throw error;
      }
    }
  }

  /**
   * Listen to specific event type
   */
  on(type: EventType, listener: EventListener): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type)!.add(listener);

    // Return unsubscribe function
    return () => {
      this.listeners.get(type)?.delete(listener);
    };
  }

  /**
   * Listen to connection state changes
   */
  onConnectionChange(listener: ConnectionListener): () => void {
    this.connectionListeners.add(listener);
    return () => this.connectionListeners.delete(listener);
  }

  /**
   * Listen to errors
   */
  onError(listener: ErrorListener): () => void {
    this.errorListeners.add(listener);
    return () => this.errorListeners.delete(listener);
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  /**
   * Get subscribed topics
   */
  getSubscribedTopics(): string[] {
    return Array.from(this.subscribedTopics);
  }

  /**
   * Remove all listeners
   */
  removeAllListeners(): void {
    this.listeners.clear();
    this.connectionListeners.clear();
    this.errorListeners.clear();
  }

  // =========================================================================
  // Private Methods
  // =========================================================================

  private handleMessage(data: string): void {
    if (typeof data !== 'string') {
      this.notifyErrorListeners(new Error('WebSocket message was not text'));
      return;
    }
    const messageBytes = new TextEncoder().encode(data).byteLength;
    if (messageBytes > MAX_WEBSOCKET_MESSAGE_BYTES) {
      this.notifyErrorListeners(
        new Error(`WebSocket message exceeds ${MAX_WEBSOCKET_MESSAGE_BYTES} bytes`)
      );
      return;
    }

    try {
      const parsed: unknown = JSON.parse(data);
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
        return;
      }
      const message = parsed as Event;
      if (typeof message.type !== 'string') {
        return;
      }

      // Emit to listeners
      if (this.listeners.has(message.type as EventType)) {
        for (const listener of Array.from(this.listeners.get(message.type as EventType) ?? [])) {
          try {
            listener(message);
          } catch (error) {
            this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
          }
        }
      }
    } catch (error) {
      this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
    }
  }

  private sendSubscription(type: 'subscribe' | 'unsubscribe', topics: EventType[]): void {
    if (topics.length === 0 || !this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    const message: WebSocketMessage = { type, data: { topics } };
    this.ws.send(JSON.stringify(message));
  }

  private notifyConnectionListeners(connected: boolean): void {
    for (const listener of Array.from(this.connectionListeners)) {
      try {
        listener(connected);
      } catch (error) {
        this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
      }
    }
  }

  private notifyErrorListeners(error: Error): void {
    for (const listener of Array.from(this.errorListeners)) {
      try {
        listener(error);
      } catch (e) {
        console.error('Error in error listener:', e);
      }
    }
  }

  private clearConnectionTimer(): void {
    if (this.connectionTimer !== null) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = null;
    }
  }

  private attemptReconnect(): void {
    if (!this.intentionallyDisconnected && this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

      this.reconnectTimer = setTimeout(() => {
        this.reconnectTimer = null;
        if (this.intentionallyDisconnected) return;
        this.connect().catch((error) => {
          this.notifyErrorListeners(error);
        });
      }, delay);
    }
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
}
