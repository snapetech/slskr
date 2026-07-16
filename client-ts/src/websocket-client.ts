/**
 * WebSocket client for real-time event streaming
 */

import { Event, EventType, WebSocketMessage } from './types';

export const websocketAuthProtocolPrefix = 'slskr.api-token.';

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
  private intentionallyDisconnected = false;
  private pingInterval: number | null = null;
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
      try {
        let settled = false;
        const socket = new WebSocket(this.url, websocketAuthProtocols(this.token));
        this.ws = socket;

        socket.onopen = () => {
          if (this.ws !== socket) return;
          try {
            this.sendSubscription('subscribe', Array.from(this.subscribedTopics));
            this.reconnectAttempts = 0;
            this.notifyConnectionListeners(true);
            this.setupPingInterval();
            settled = true;
            resolve();
          } catch (error) {
            settled = true;
            reject(error);
            socket.close();
          }
        };

        socket.onmessage = (event) => {
          if (this.ws !== socket) return;
          this.handleMessage(event.data);
        };

        socket.onerror = () => {
          if (this.ws !== socket) return;
          this.notifyErrorListeners(new Error('WebSocket error'));
          settled = true;
          reject(new Error('WebSocket connection error'));
        };

        socket.onclose = () => {
          if (this.ws !== socket) return;
          this.ws = null;
          this.notifyConnectionListeners(false);
          this.clearPingInterval();
          if (!settled) {
            settled = true;
            reject(new Error('WebSocket closed before opening'));
          }
          if (!this.intentionallyDisconnected) {
            this.attemptReconnect();
          }
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Disconnect from WebSocket
   */
  disconnect(): void {
    this.intentionallyDisconnected = true;
    this.clearReconnectTimer();
    this.clearPingInterval();
    if (this.ws) {
      this.ws.close();
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
    try {
      const message = JSON.parse(data) as Event;

      // Emit to listeners
      if (this.listeners.has(message.type as EventType)) {
        this.listeners.get(message.type as EventType)?.forEach((listener) => {
          try {
            listener(message);
          } catch (error) {
            this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
          }
        });
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
    this.connectionListeners.forEach((listener) => {
      try {
        listener(connected);
      } catch (error) {
        this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  private notifyErrorListeners(error: Error): void {
    this.errorListeners.forEach((listener) => {
      try {
        listener(error);
      } catch (e) {
        console.error('Error in error listener:', e);
      }
    });
  }

  private setupPingInterval(): void {
    this.pingInterval = globalThis.setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        const message: WebSocketMessage = { type: 'ping' };
        this.ws.send(JSON.stringify(message));
      }
    }, 30000) as unknown as number; // 30 seconds
  }

  private clearPingInterval(): void {
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
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
