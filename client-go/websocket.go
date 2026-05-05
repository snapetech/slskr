package slskr

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"

	"github.com/gorilla/websocket"
)

const maxWebSocketMessageBytes = 64 * 1024

// WebSocketClient represents a WebSocket connection to the API
type WebSocketClient struct {
	url              string
	token            string
	debug            bool
	mu               sync.RWMutex
	connected        bool
	subscriptionMu   sync.RWMutex
	subscribedTopics map[string]bool
	conn             *websocket.Conn

	// Channels for events
	eventChannels map[string][]chan interface{}
	connectionCh  []chan bool
	errorCh       []chan error
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
	}
}

// Connect connects to the WebSocket
func (w *WebSocketClient) Connect(ctx context.Context) error {
	w.mu.Lock()
	if w.connected {
		w.mu.Unlock()
		return fmt.Errorf("already connected")
	}
	w.mu.Unlock()

	headers := http.Header{}
	if w.token != "" {
		headers.Set("Authorization", "Bearer "+w.token)
	}

	conn, _, err := websocket.DefaultDialer.DialContext(ctx, w.url, headers)
	if err != nil {
		return err
	}
	conn.SetReadLimit(maxWebSocketMessageBytes)

	w.mu.Lock()
	w.conn = conn
	w.connected = true
	w.mu.Unlock()

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
	if !w.connected {
		w.mu.Unlock()
		return fmt.Errorf("not connected")
	}

	w.connected = false
	conn := w.conn
	w.conn = nil
	w.mu.Unlock()

	if conn != nil {
		_ = conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
		_ = conn.Close()
	}

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
		if err := w.writeJSON(msg); err != nil {
			return err
		}
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
	for {
		w.mu.RLock()
		conn := w.conn
		w.mu.RUnlock()
		if conn == nil {
			return
		}

		_, data, err := conn.ReadMessage()
		if err != nil {
			w.mu.Lock()
			wasConnected := w.connected
			w.connected = false
			w.conn = nil
			w.mu.Unlock()

			if wasConnected {
				w.notifyConnectionListeners(false)
				w.notifyErrorListeners(err)
			}
			return
		}

		msg, err := parseMessage(data)
		if err != nil {
			w.notifyErrorListeners(err)
			continue
		}
		w.processMessage(msg)
	}
}

func (w *WebSocketClient) writeJSON(msg map[string]interface{}) error {
	w.mu.RLock()
	conn := w.conn
	w.mu.RUnlock()

	if conn == nil {
		return fmt.Errorf("not connected")
	}

	return conn.WriteJSON(msg)
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
