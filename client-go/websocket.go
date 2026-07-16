package slskr

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"path"
	"sync"

	"github.com/gorilla/websocket"
)

const maxWebSocketMessageBytes = 64 * 1024

// WebSocketClient represents a WebSocket connection to the API
type WebSocketClient struct {
	url               string
	initErr           error
	token             string
	debug             bool
	mu                sync.RWMutex
	connected         bool
	connecting        bool
	disconnectPending bool
	writeMu           sync.Mutex
	subscriptionMu    sync.RWMutex
	subscribedTopics  map[string]bool
	conn              *websocket.Conn

	// Channels for events
	eventChannels map[string][]chan interface{}
	connectionCh  []chan bool
	errorCh       []chan error
}

// NewWebSocketClient creates a new WebSocket client
func (c *Client) NewWebSocketClient(debug bool) *WebSocketClient {
	wsURL, err := websocketURL(c.BaseURL)

	return &WebSocketClient{
		url:              wsURL,
		initErr:          err,
		token:            c.Token,
		debug:            debug,
		subscribedTopics: make(map[string]bool),
		eventChannels:    make(map[string][]chan interface{}),
	}
}

// Connect connects to the WebSocket
func (w *WebSocketClient) Connect(ctx context.Context) error {
	w.mu.Lock()
	if w.initErr != nil {
		err := w.initErr
		w.mu.Unlock()
		return err
	}
	if w.connected {
		w.mu.Unlock()
		return fmt.Errorf("already connected")
	}
	if w.connecting {
		w.mu.Unlock()
		return fmt.Errorf("connection already in progress")
	}
	w.connecting = true
	w.disconnectPending = false
	w.mu.Unlock()

	headers := http.Header{}
	if w.token != "" {
		headers.Set("Authorization", "Bearer "+w.token)
	}

	conn, _, err := websocket.DefaultDialer.DialContext(ctx, w.url, headers)
	if err != nil {
		w.mu.Lock()
		w.connecting = false
		w.mu.Unlock()
		return err
	}
	conn.SetReadLimit(maxWebSocketMessageBytes)

	w.subscriptionMu.Lock()
	retainedTopics := make([]string, 0, len(w.subscribedTopics))
	for topic := range w.subscribedTopics {
		retainedTopics = append(retainedTopics, topic)
	}
	if len(retainedTopics) > 0 {
		msg := map[string]interface{}{
			"type": "subscribe",
			"data": map[string]interface{}{"topics": retainedTopics},
		}
		w.writeMu.Lock()
		err := conn.WriteJSON(msg)
		w.writeMu.Unlock()
		if err != nil {
			w.subscriptionMu.Unlock()
			w.mu.Lock()
			w.connecting = false
			w.mu.Unlock()
			_ = conn.Close()
			return fmt.Errorf("restore subscriptions: %w", err)
		}
	}

	w.mu.Lock()
	w.connecting = false
	if w.disconnectPending {
		w.disconnectPending = false
		w.mu.Unlock()
		w.subscriptionMu.Unlock()
		_ = conn.Close()
		return fmt.Errorf("connection canceled by disconnect")
	}
	w.conn = conn
	w.connected = true
	w.mu.Unlock()
	w.subscriptionMu.Unlock()

	if w.debug {
		fmt.Printf("[WebSocket] Connected to %s\n", w.url)
	}

	// Notify connection listeners
	w.notifyConnectionListeners(true)

	// Start message handler
	go w.handleMessages(conn)

	return nil
}

func websocketURL(baseURL string) (string, error) {
	parsed, err := url.Parse(baseURL)
	if err != nil || parsed.Host == "" || (parsed.Scheme != "http" && parsed.Scheme != "https") {
		return "", fmt.Errorf("base URL must be an absolute HTTP or HTTPS URL")
	}
	if parsed.Scheme == "https" {
		parsed.Scheme = "wss"
	} else {
		parsed.Scheme = "ws"
	}
	parsed.Path = path.Join(parsed.Path, "/api/events/ws")
	parsed.RawQuery = ""
	parsed.Fragment = ""
	return parsed.String(), nil
}

// Disconnect closes the WebSocket connection
func (w *WebSocketClient) Disconnect(ctx context.Context) error {
	w.mu.Lock()
	if !w.connected {
		if w.connecting {
			w.disconnectPending = true
			w.mu.Unlock()
			return nil
		}
		w.mu.Unlock()
		return fmt.Errorf("not connected")
	}

	w.connected = false
	conn := w.conn
	w.conn = nil
	w.mu.Unlock()

	if conn != nil {
		w.writeMu.Lock()
		_ = conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
		_ = conn.Close()
		w.writeMu.Unlock()
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
	newTopics := make([]string, 0, len(topics))
	for _, topic := range topics {
		if w.subscribedTopics[topic] {
			continue
		}
		w.subscribedTopics[topic] = true
		newTopics = append(newTopics, topic)
	}
	if len(newTopics) == 0 {
		return nil
	}

	if w.debug {
		fmt.Printf("[WebSocket] Subscribed to: %v\n", newTopics)
	}

	// Send subscription message
	msg := map[string]interface{}{
		"type": "subscribe",
		"data": map[string]interface{}{
			"topics": newTopics,
		},
	}

	if w.IsConnected() {
		if err := w.writeJSON(msg); err != nil {
			for _, topic := range newTopics {
				delete(w.subscribedTopics, topic)
			}
			return err
		}
	}

	return nil
}

// Unsubscribe unsubscribes from event topics
func (w *WebSocketClient) Unsubscribe(topics ...string) error {
	w.subscriptionMu.Lock()
	defer w.subscriptionMu.Unlock()
	removedTopics := make([]string, 0, len(topics))
	for _, topic := range topics {
		if !w.subscribedTopics[topic] {
			continue
		}
		delete(w.subscribedTopics, topic)
		removedTopics = append(removedTopics, topic)
	}
	if len(removedTopics) == 0 {
		return nil
	}

	if w.debug {
		fmt.Printf("[WebSocket] Unsubscribed from: %v\n", removedTopics)
	}

	msg := map[string]interface{}{
		"type": "unsubscribe",
		"data": map[string]interface{}{
			"topics": removedTopics,
		},
	}
	if w.IsConnected() {
		if err := w.writeJSON(msg); err != nil {
			for _, topic := range removedTopics {
				w.subscribedTopics[topic] = true
			}
			return err
		}
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

func (w *WebSocketClient) handleMessages(conn *websocket.Conn) {
	for {
		_, data, err := conn.ReadMessage()
		if err != nil {
			if w.clearConnectionIfCurrent(conn) {
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

func (w *WebSocketClient) clearConnectionIfCurrent(conn *websocket.Conn) bool {
	w.mu.Lock()
	defer w.mu.Unlock()
	if !w.connected || w.conn != conn {
		return false
	}
	w.connected = false
	w.conn = nil
	return true
}

func (w *WebSocketClient) writeJSON(msg map[string]interface{}) error {
	w.writeMu.Lock()
	defer w.writeMu.Unlock()

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
