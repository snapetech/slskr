/**
 * WebSocket client for real-time event streaming
 */
import { Event, EventType } from './types';
export declare const websocketAuthProtocolPrefix = "slskr.api-token.";
export declare function websocketAuthProtocols(token: string): string[];
export type EventListener = (event: Event) => void;
export type ConnectionListener = (connected: boolean) => void;
export type ErrorListener = (error: Error) => void;
export declare class WebSocketClient {
    private ws;
    private url;
    private token;
    private reconnectAttempts;
    private maxReconnectAttempts;
    private reconnectDelay;
    private pingInterval;
    private subscribedTopics;
    private listeners;
    private connectionListeners;
    private errorListeners;
    constructor(baseUrl: string, token: string);
    /**
     * Connect to WebSocket
     */
    connect(): Promise<void>;
    /**
     * Disconnect from WebSocket
     */
    disconnect(): void;
    /**
     * Subscribe to event types
     */
    subscribe(...topics: EventType[]): void;
    /**
     * Unsubscribe from event types
     */
    unsubscribe(...topics: EventType[]): void;
    /**
     * Listen to specific event type
     */
    on(type: EventType, listener: EventListener): () => void;
    /**
     * Listen to connection state changes
     */
    onConnectionChange(listener: ConnectionListener): () => void;
    /**
     * Listen to errors
     */
    onError(listener: ErrorListener): () => void;
    /**
     * Check if connected
     */
    isConnected(): boolean;
    /**
     * Get subscribed topics
     */
    getSubscribedTopics(): string[];
    /**
     * Remove all listeners
     */
    removeAllListeners(): void;
    private handleMessage;
    private notifyConnectionListeners;
    private notifyErrorListeners;
    private setupPingInterval;
    private clearPingInterval;
    private attemptReconnect;
}
//# sourceMappingURL=websocket-client.d.ts.map