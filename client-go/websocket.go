package slskr

import (
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"path"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

const (
	maxWebSocketMessageBytes       = 64 * 1024
	defaultWebSocketConnectTimeout = 30 * time.Second
	webSocketReadTimeout           = 2 * time.Minute
)

// WebSocketClient represents a WebSocket connection to the API
type WebSocketClient struct {
	url                       string
	initErr                   error
	token                     string
	debug                     bool
	connectTimeout            time.Duration
	mu                        sync.RWMutex
	connected                 bool
	connecting                bool
	disconnectPending         bool
	intentionallyDisconnected bool
	connectCancel             context.CancelFunc
	connectingNetConn         net.Conn
	connectingConn            *websocket.Conn
	reconnectAttempts         int
	maxReconnectAttempts      int
	reconnectDelay            time.Duration
	reconnectTimer            *time.Timer
	writeMu                   sync.Mutex
	subscriptionMu            sync.RWMutex
	subscribedTopics          map[string]bool
	conn                      *websocket.Conn

	// Channels for events
	eventChannels map[string][]chan interface{}
	connectionCh  []chan bool
	errorCh       []chan error
}

// NewWebSocketClient creates a new WebSocket client
func (c *Client) NewWebSocketClient(debug bool) *WebSocketClient {
	wsURL, err := websocketURL(c.BaseURL)
	connectTimeout := c.Timeout
	if connectTimeout <= 0 {
		connectTimeout = defaultWebSocketConnectTimeout
	}

	return &WebSocketClient{
		url:                  wsURL,
		initErr:              err,
		token:                c.Token,
		debug:                debug,
		connectTimeout:       connectTimeout,
		maxReconnectAttempts: 5,
		reconnectDelay:       time.Second,
		subscribedTopics:     make(map[string]bool),
		eventChannels:        make(map[string][]chan interface{}),
	}
}

// Connect connects to the WebSocket
func (w *WebSocketClient) Connect(ctx context.Context) error {
	return w.connect(ctx, false)
}

func (w *WebSocketClient) connect(ctx context.Context, automatic bool) error {
	w.mu.Lock()
	if w.initErr != nil {
		err := w.initErr
		w.mu.Unlock()
		return err
	}
	if w.connected {
		w.mu.Unlock()
		if automatic {
			return context.Canceled
		}
		return fmt.Errorf("already connected")
	}
	if w.connecting {
		w.mu.Unlock()
		if automatic {
			return context.Canceled
		}
		return fmt.Errorf("connection already in progress")
	}
	if automatic && w.intentionallyDisconnected {
		w.mu.Unlock()
		return context.Canceled
	}
	reconnectTimer := w.reconnectTimer
	w.reconnectTimer = nil
	w.connecting = true
	w.disconnectPending = false
	if !automatic {
		w.intentionallyDisconnected = false
	}
	w.mu.Unlock()
	if reconnectTimer != nil {
		reconnectTimer.Stop()
	}

	headers := http.Header{}
	if w.token != "" {
		headers.Set("Authorization", "Bearer "+w.token)
	}

	dialContext, cancelDial := context.WithTimeout(ctx, w.connectTimeout)
	w.mu.Lock()
	w.connectCancel = cancelDial
	disconnectPending := w.disconnectPending
	w.mu.Unlock()
	if disconnectPending {
		cancelDial()
	}
	dialer := *websocket.DefaultDialer
	baseNetDialContext := dialer.NetDialContext
	baseNetDial := dialer.NetDial
	baseNetDialTLSContext := dialer.NetDialTLSContext
	trackConnectingConn := func(conn net.Conn) (net.Conn, error) {
		w.mu.Lock()
		disconnectPending := w.disconnectPending
		if !disconnectPending {
			w.connectingNetConn = conn
		}
		w.mu.Unlock()
		if disconnectPending {
			_ = conn.Close()
			return nil, context.Canceled
		}
		return conn, nil
	}
	dialer.NetDialContext = func(dialCtx context.Context, network, address string) (net.Conn, error) {
		var (
			conn net.Conn
			err  error
		)
		switch {
		case baseNetDialContext != nil:
			conn, err = baseNetDialContext(dialCtx, network, address)
		case baseNetDial != nil:
			conn, err = baseNetDial(network, address)
		default:
			conn, err = (&net.Dialer{}).DialContext(dialCtx, network, address)
		}
		if err != nil {
			return nil, err
		}
		return trackConnectingConn(conn)
	}
	if baseNetDialTLSContext != nil {
		dialer.NetDialTLSContext = func(dialCtx context.Context, network, address string) (net.Conn, error) {
			conn, err := baseNetDialTLSContext(dialCtx, network, address)
			if err != nil {
				return nil, err
			}
			return trackConnectingConn(conn)
		}
	}
	conn, _, err := dialer.DialContext(dialContext, w.url, headers)
	cancelDial()
	w.mu.Lock()
	w.connectingNetConn = nil
	disconnectPending = w.disconnectPending
	w.mu.Unlock()
	if err != nil {
		w.mu.Lock()
		w.connecting = false
		w.connectCancel = nil
		w.disconnectPending = false
		w.mu.Unlock()
		return err
	}
	conn.SetReadLimit(maxWebSocketMessageBytes)
	w.mu.Lock()
	w.connectingConn = conn
	disconnectPending = w.disconnectPending
	w.mu.Unlock()
	if disconnectPending {
		_ = conn.Close()
	}

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
		err := w.writeJSONOnConnection(conn, msg, websocketDeadline(ctx, w.connectTimeout))
		w.writeMu.Unlock()
		if err != nil {
			w.subscriptionMu.Unlock()
			w.mu.Lock()
			w.connecting = false
			w.connectCancel = nil
			w.connectingNetConn = nil
			w.connectingConn = nil
			w.mu.Unlock()
			_ = conn.Close()
			return fmt.Errorf("restore subscriptions: %w", err)
		}
	}
	_ = conn.SetReadDeadline(time.Now().Add(webSocketReadTimeout))
	conn.SetPongHandler(func(string) error {
		return conn.SetReadDeadline(time.Now().Add(webSocketReadTimeout))
	})

	w.mu.Lock()
	w.connecting = false
	w.reconnectAttempts = 0
	if w.disconnectPending {
		w.disconnectPending = false
		w.connectCancel = nil
		w.connectingNetConn = nil
		w.connectingConn = nil
		w.mu.Unlock()
		w.subscriptionMu.Unlock()
		_ = conn.Close()
		return fmt.Errorf("connection canceled by disconnect")
	}
	w.connectingConn = nil
	w.connectCancel = nil
	w.connectingNetConn = nil
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
	w.intentionallyDisconnected = true
	reconnectTimer := w.reconnectTimer
	w.reconnectTimer = nil
	if !w.connected {
		if w.connecting {
			w.disconnectPending = true
			cancelConnect := w.connectCancel
			connectingNetConn := w.connectingNetConn
			connectingConn := w.connectingConn
			w.mu.Unlock()
			if reconnectTimer != nil {
				reconnectTimer.Stop()
			}
			if cancelConnect != nil {
				cancelConnect()
			}
			if connectingNetConn != nil {
				_ = connectingNetConn.Close()
			}
			if connectingConn != nil {
				_ = connectingConn.Close()
			}
			return nil
		}
		w.mu.Unlock()
		if reconnectTimer != nil {
			reconnectTimer.Stop()
		}
		return fmt.Errorf("not connected")
	}

	w.connected = false
	conn := w.conn
	w.conn = nil
	w.mu.Unlock()
	if reconnectTimer != nil {
		reconnectTimer.Stop()
	}

	if conn != nil {
		w.writeMu.Lock()
		_ = conn.SetWriteDeadline(websocketDeadline(ctx, w.connectTimeout))
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
				w.scheduleReconnect()
			}
			return
		}
		_ = conn.SetReadDeadline(time.Now().Add(webSocketReadTimeout))

		msg, err := parseMessage(data)
		if err != nil {
			w.notifyErrorListeners(err)
			continue
		}
		w.processMessage(msg)
	}
}

func (w *WebSocketClient) scheduleReconnect() {
	w.mu.Lock()
	if w.intentionallyDisconnected || w.connected || w.connecting ||
		w.reconnectTimer != nil || w.reconnectAttempts >= w.maxReconnectAttempts {
		w.mu.Unlock()
		return
	}
	w.reconnectAttempts++
	attempt := w.reconnectAttempts
	delay := w.reconnectDelay
	if delay < 0 {
		delay = 0
	}
	for step := 1; step < attempt; step++ {
		if delay >= 15*time.Second {
			delay = 30 * time.Second
			break
		}
		delay *= 2
	}
	if delay > 30*time.Second {
		delay = 30 * time.Second
	}
	w.reconnectTimer = time.AfterFunc(delay, w.runReconnect)
	w.mu.Unlock()
}

func (w *WebSocketClient) runReconnect() {
	w.mu.Lock()
	w.reconnectTimer = nil
	shouldReconnect := !w.intentionallyDisconnected && !w.connected && !w.connecting
	w.mu.Unlock()
	if !shouldReconnect {
		return
	}
	if err := w.connect(context.Background(), true); err != nil {
		if err == context.Canceled {
			return
		}
		w.mu.RLock()
		intentionallyDisconnected := w.intentionallyDisconnected
		w.mu.RUnlock()
		if intentionallyDisconnected {
			return
		}
		w.notifyErrorListeners(err)
		w.scheduleReconnect()
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

	return w.writeJSONOnConnection(conn, msg, time.Now().Add(w.connectTimeout))
}

func (w *WebSocketClient) writeJSONOnConnection(conn *websocket.Conn, msg interface{}, deadline time.Time) error {
	if err := conn.SetWriteDeadline(deadline); err != nil {
		return err
	}
	return conn.WriteJSON(msg)
}

func websocketDeadline(ctx context.Context, timeout time.Duration) time.Time {
	deadline := time.Now().Add(timeout)
	if ctxDeadline, ok := ctx.Deadline(); ok && ctxDeadline.Before(deadline) {
		return ctxDeadline
	}
	return deadline
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
