"use strict";
/**
 * WebSocket client for real-time event streaming
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.WebSocketClient = exports.WEBSOCKET_CONNECT_TIMEOUT_MS = exports.websocketAuthProtocolPrefix = void 0;
exports.websocketAuthProtocols = websocketAuthProtocols;
exports.websocketAuthProtocolPrefix = 'slskr.api-token.';
exports.WEBSOCKET_CONNECT_TIMEOUT_MS = 15000;
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
        this.reconnectTimer = null;
        this.connectionTimer = null;
        this.pendingConnectReject = null;
        this.intentionallyDisconnected = false;
        this.subscribedTopics = new Set();
        this.listeners = new Map();
        this.connectionListeners = new Set();
        this.errorListeners = new Set();
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
    async connect() {
        if (this.ws !== null) {
            throw new Error(this.ws.readyState === WebSocket.OPEN
                ? 'WebSocket is already connected'
                : 'WebSocket connection already in progress');
        }
        this.intentionallyDisconnected = false;
        this.clearReconnectTimer();
        return new Promise((resolve, reject) => {
            let settled = false;
            const clearConnectionTimer = () => this.clearConnectionTimer();
            const settle = (callback) => {
                if (settled)
                    return false;
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
                    if (this.ws !== socket || settled)
                        return;
                    const error = new Error(`WebSocket connection timed out after ${exports.WEBSOCKET_CONNECT_TIMEOUT_MS}ms`);
                    this.notifyErrorListeners(error);
                    settle(() => reject(error));
                    socket.close();
                }, exports.WEBSOCKET_CONNECT_TIMEOUT_MS);
                socket.onopen = () => {
                    if (this.ws !== socket)
                        return;
                    if (settled)
                        return;
                    clearConnectionTimer();
                    try {
                        this.sendSubscription('subscribe', Array.from(this.subscribedTopics));
                        this.reconnectAttempts = 0;
                        this.notifyConnectionListeners(true);
                        settle(resolve);
                    }
                    catch (error) {
                        settle(() => reject(error instanceof Error ? error : new Error(String(error))));
                        socket.close();
                    }
                };
                socket.onmessage = (event) => {
                    if (this.ws !== socket)
                        return;
                    this.handleMessage(event.data);
                };
                socket.onerror = () => {
                    if (this.ws !== socket)
                        return;
                    const error = new Error('WebSocket error');
                    this.notifyErrorListeners(error);
                    if (!settled && socket.readyState !== WebSocket.OPEN) {
                        settle(() => reject(new Error('WebSocket connection error')));
                        socket.close();
                    }
                };
                socket.onclose = () => {
                    if (this.ws !== socket)
                        return;
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
            }
            catch (error) {
                settle(() => reject(error instanceof Error ? error : new Error(String(error))));
            }
        });
    }
    /**
     * Disconnect from WebSocket
     */
    disconnect() {
        this.intentionallyDisconnected = true;
        this.clearReconnectTimer();
        this.clearConnectionTimer();
        this.pendingConnectReject?.(new Error('WebSocket closed before opening'));
        if (this.ws) {
            this.ws.close();
        }
    }
    /**
     * Subscribe to event types
     */
    subscribe(...topics) {
        const newTopics = topics.filter((topic, index) => topics.indexOf(topic) === index && !this.subscribedTopics.has(topic));
        if (newTopics.length === 0)
            return;
        newTopics.forEach((t) => this.subscribedTopics.add(t));
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            try {
                this.sendSubscription('subscribe', newTopics);
            }
            catch (error) {
                newTopics.forEach((topic) => this.subscribedTopics.delete(topic));
                throw error;
            }
        }
    }
    /**
     * Unsubscribe from event types
     */
    unsubscribe(...topics) {
        const removedTopics = topics.filter((topic, index) => topics.indexOf(topic) === index && this.subscribedTopics.delete(topic));
        if (removedTopics.length === 0)
            return;
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            try {
                this.sendSubscription('unsubscribe', removedTopics);
            }
            catch (error) {
                removedTopics.forEach((topic) => this.subscribedTopics.add(topic));
                throw error;
            }
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
    sendSubscription(type, topics) {
        if (topics.length === 0 || !this.ws || this.ws.readyState !== WebSocket.OPEN)
            return;
        const message = { type, data: { topics } };
        this.ws.send(JSON.stringify(message));
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
    clearConnectionTimer() {
        if (this.connectionTimer !== null) {
            clearTimeout(this.connectionTimer);
            this.connectionTimer = null;
        }
    }
    attemptReconnect() {
        if (!this.intentionallyDisconnected && this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
            this.reconnectTimer = setTimeout(() => {
                this.reconnectTimer = null;
                if (this.intentionallyDisconnected)
                    return;
                this.connect().catch((error) => {
                    this.notifyErrorListeners(error);
                });
            }, delay);
        }
    }
    clearReconnectTimer() {
        if (this.reconnectTimer !== null) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
    }
}
exports.WebSocketClient = WebSocketClient;
//# sourceMappingURL=websocket-client.js.map