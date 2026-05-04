package soulseekr

import (
	"context"
	"encoding/json"
	"fmt"
	"net/url"
	"strings"
	"sync"
)

// WebSocketClient represents a WebSocket connection to the API
type WebSocketClient struct {
	url              string
	token            string
	debug            bool
	mu               sync.RWMutex
	connected        bool
	subscriptionMu   sync.RWMutex
	subscribedTopics map[string]bool

	// Channels for events
	eventChannels map[string][]chan interface{}
	connectionCh  []chan bool
	errorCh       []chan error

	// Mock connection state (in production, would use actual WebSocket)
	mockMessages chan map[string]interface{}
}

// NewWebSocketClient creates a new WebSocket client
func (c *Client) NewWebSocketClient(debug bool) *WebSocketClient {
	wsURL := strings.Replace(c.BaseURL, "http", "ws", 1)
	wsURL = strings.TrimRight(wsURL, "/") + "/api/events/ws"

	return &WebSocketClient{
		url:              wsURL,
		token:            c.Token,
		debug:            debug,
		subscribedTopics: make(map[string]bool),
		eventChannels:    make(map[string][]chan interface{}),
		mockMessages:     make(chan map[string]interface{}, 100),
	}
}

// Connect connects to the WebSocket
func (w *WebSocketClient) Connect(ctx context.Context) error {
	w.mu.Lock()
	defer w.mu.Unlock()

	if w.connected {
		return fmt.Errorf("already connected")
	}

	// In a real implementation, this would establish a WebSocket connection
	// For now, we simulate with channels
	w.connected = true

	if w.debug {
		fmt.Printf("[WebSocket] Connected to %s\n", w.url)
	}

	// Notify connection listeners
	w.notifyConnectionListeners(true)

	// Start message handler
	go w.handleMessages()

	return nil
}

// Disconnect closes the WebSocket connection
func (w *WebSocketClient) Disconnect(ctx context.Context) error {
	w.mu.Lock()
	defer w.mu.Unlock()

	if !w.connected {
		return fmt.Errorf("not connected")
	}

	w.connected = false
	close(w.mockMessages)

	if w.debug {
		fmt.Println("[WebSocket] Disconnected")
	}

	w.notifyConnectionListeners(false)
	return nil
}

// IsConnected returns connection state
func (w *WebSocketClient) IsConnected() bool {
	w.mu.RLock()
	defer w.mu.RUnlock()
	return w.connected
}

// Subscribe subscribes to event topics
func (w *WebSocketClient) Subscribe(topics ...string) error {
	w.subscriptionMu.Lock()
	defer w.subscriptionMu.Unlock()

	for _, topic := range topics {
		w.subscribedTopics[topic] = true
	}

	if w.debug {
		fmt.Printf("[WebSocket] Subscribed to: %v\n", topics)
	}

	// Send subscription message
	msg := map[string]interface{}{
		"type": "subscribe",
		"data": map[string]interface{}{
			"topics": topics,
		},
	}

	if w.IsConnected() {
		w.mockMessages <- msg
	}

	return nil
}

// Unsubscribe unsubscribes from event topics
func (w *WebSocketClient) Unsubscribe(topics ...string) error {
	w.subscriptionMu.Lock()
	defer w.subscriptionMu.Unlock()

	for _, topic := range topics {
		delete(w.subscribedTopics, topic)
	}

	if w.debug {
		fmt.Printf("[WebSocket] Unsubscribed from: %v\n", topics)
	}

	return nil
}

// GetSubscribedTopics returns list of subscribed topics
func (w *WebSocketClient) GetSubscribedTopics() []string {
	w.subscriptionMu.RLock()
	defer w.subscriptionMu.RUnlock()

	topics := make([]string, 0, len(w.subscribedTopics))
	for topic := range w.subscribedTopics {
		topics = append(topics, topic)
	}
	return topics
}

// On registers an event listener
func (w *WebSocketClient) On(eventType string, ch chan interface{}) {
	w.mu.Lock()
	defer w.mu.Unlock()

	if w.eventChannels[eventType] == nil {
		w.eventChannels[eventType] = []chan interface{}{}
	}
	w.eventChannels[eventType] = append(w.eventChannels[eventType], ch)
}

// OnConnectionChange registers a connection state listener
func (w *WebSocketClient) OnConnectionChange(ch chan bool) {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.connectionCh = append(w.connectionCh, ch)
}

// OnError registers an error listener
func (w *WebSocketClient) OnError(ch chan error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.errorCh = append(w.errorCh, ch)
}

// ============================================================================
// Private Methods
// ============================================================================

func (w *WebSocketClient) handleMessages() {
	for msg := range w.mockMessages {
		w.processMessage(msg)
	}
}

func (w *WebSocketClient) processMessage(msg map[string]interface{}) {
	eventType := ""
	if t, ok := msg["type"].(string); ok {
		eventType = t
	}

	w.mu.RLock()
	listeners := w.eventChannels[eventType]
	w.mu.RUnlock()

	for _, ch := range listeners {
		select {
		case ch <- msg:
		default:
			// Channel full, skip
		}
	}
}

func (w *WebSocketClient) notifyConnectionListeners(connected bool) {
	w.mu.RLock()
	listeners := make([]chan bool, len(w.connectionCh))
	copy(listeners, w.connectionCh)
	w.mu.RUnlock()

	for _, ch := range listeners {
		select {
		case ch <- connected:
		default:
			// Channel full, skip
		}
	}
}

func (w *WebSocketClient) notifyErrorListeners(err error) {
	w.mu.RLock()
	listeners := make([]chan error, len(w.errorCh))
	copy(listeners, w.errorCh)
	w.mu.RUnlock()

	for _, ch := range listeners {
		select {
		case ch <- err:
		default:
			// Channel full, skip
		}
	}
}

// Message creates a JSON-encoded message
func encodeMessage(data interface{}) ([]byte, error) {
	return json.Marshal(data)
}

// ParseMessage decodes a JSON message
func parseMessage(data []byte) (map[string]interface{}, error) {
	var result map[string]interface{}
	err := json.Unmarshal(data, &result)
	return result, err
}
