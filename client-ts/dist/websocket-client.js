"use strict";
/**
 * WebSocket client for real-time event streaming
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.WebSocketClient = exports.websocketAuthProtocolPrefix = void 0;
exports.websocketAuthProtocols = websocketAuthProtocols;
exports.websocketAuthProtocolPrefix = 'slskr.api-token.';
function websocketAuthProtocols(token) {
    const normalized = token.trim();
    return normalized ? [`${exports.websocketAuthProtocolPrefix}${encodeURIComponent(normalized)}`] : [];
}
class WebSocketClient {
    constructor(baseUrl, token) {
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;
        this.pingInterval = null;
        this.subscribedTopics = new Set();
        this.listeners = new Map();
        this.connectionListeners = new Set();
        this.errorListeners = new Set();
        this.url = baseUrl
            .replace(/^http/, 'ws')
            .replace(/\/$/, '') + '/api/events/ws';
        this.token = token;
    }
    /**
     * Connect to WebSocket
     */
    async connect() {
        return new Promise((resolve, reject) => {
            try {
                this.ws = new WebSocket(this.url, websocketAuthProtocols(this.token));
                this.ws.onopen = () => {
                    this.reconnectAttempts = 0;
                    this.notifyConnectionListeners(true);
                    this.setupPingInterval();
                    resolve();
                };
                this.ws.onmessage = (event) => {
                    this.handleMessage(event.data);
                };
                this.ws.onerror = () => {
                    this.notifyErrorListeners(new Error('WebSocket error'));
                    reject(new Error('WebSocket connection error'));
                };
                this.ws.onclose = () => {
                    this.notifyConnectionListeners(false);
                    this.clearPingInterval();
                    this.attemptReconnect();
                };
            }
            catch (error) {
                reject(error);
            }
        });
    }
    /**
     * Disconnect from WebSocket
     */
    disconnect() {
        this.clearPingInterval();
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
    /**
     * Subscribe to event types
     */
    subscribe(...topics) {
        const newTopics = topics.filter((t) => !this.subscribedTopics.has(t));
        if (newTopics.length === 0)
            return;
        newTopics.forEach((t) => this.subscribedTopics.add(t));
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            const message = {
                type: 'subscribe',
                data: { topics: newTopics },
            };
            this.ws.send(JSON.stringify(message));
        }
    }
    /**
     * Unsubscribe from event types
     */
    unsubscribe(...topics) {
        topics.forEach((t) => this.subscribedTopics.delete(t));
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            const message = {
                type: 'unsubscribe',
                data: { topics },
            };
            this.ws.send(JSON.stringify(message));
        }
    }
    /**
     * Listen to specific event type
     */
    on(type, listener) {
        if (!this.listeners.has(type)) {
            this.listeners.set(type, new Set());
        }
        this.listeners.get(type).add(listener);
        // Return unsubscribe function
        return () => {
            this.listeners.get(type)?.delete(listener);
        };
    }
    /**
     * Listen to connection state changes
     */
    onConnectionChange(listener) {
        this.connectionListeners.add(listener);
        return () => this.connectionListeners.delete(listener);
    }
    /**
     * Listen to errors
     */
    onError(listener) {
        this.errorListeners.add(listener);
        return () => this.errorListeners.delete(listener);
    }
    /**
     * Check if connected
     */
    isConnected() {
        return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }
    /**
     * Get subscribed topics
     */
    getSubscribedTopics() {
        return Array.from(this.subscribedTopics);
    }
    /**
     * Remove all listeners
     */
    removeAllListeners() {
        this.listeners.clear();
        this.connectionListeners.clear();
        this.errorListeners.clear();
    }
    // =========================================================================
    // Private Methods
    // =========================================================================
    handleMessage(data) {
        try {
            const message = JSON.parse(data);
            // Emit to listeners
            if (this.listeners.has(message.type)) {
                this.listeners.get(message.type)?.forEach((listener) => {
                    try {
                        listener(message);
                    }
                    catch (error) {
                        this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
                    }
                });
            }
        }
        catch (error) {
            this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
        }
    }
    notifyConnectionListeners(connected) {
        this.connectionListeners.forEach((listener) => {
            try {
                listener(connected);
            }
            catch (error) {
                this.notifyErrorListeners(error instanceof Error ? error : new Error(String(error)));
            }
        });
    }
    notifyErrorListeners(error) {
        this.errorListeners.forEach((listener) => {
            try {
                listener(error);
            }
            catch (e) {
                console.error('Error in error listener:', e);
            }
        });
    }
    setupPingInterval() {
        this.pingInterval = globalThis.setInterval(() => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                const message = { type: 'ping' };
                this.ws.send(JSON.stringify(message));
            }
        }, 30000); // 30 seconds
    }
    clearPingInterval() {
        if (this.pingInterval !== null) {
            clearInterval(this.pingInterval);
            this.pingInterval = null;
        }
    }
    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
            setTimeout(() => {
                this.connect().catch((error) => {
                    this.notifyErrorListeners(error);
                });
            }, delay);
        }
    }
}
exports.WebSocketClient = WebSocketClient;
//# sourceMappingURL=websocket-client.js.map