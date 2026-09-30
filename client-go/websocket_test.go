package slskr

import (
	"bufio"
	"bytes"
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

func TestWebSocketURLRejectsCredentials(t *testing.T) {
	for _, baseURL := range []string{
		"https://user@example.test",
		"https://user:password@example.test",
	} {
		if _, err := websocketURL(baseURL); err == nil {
			t.Fatalf("expected credentials to be rejected for %q", baseURL)
		}
	}
}

func TestWebSocketSubscriptionFramesTrackLocalState(t *testing.T) {
	frames := make(chan map[string]interface{}, 2)
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err != nil {
			return
		}
		defer connection.Close()
		for index := 0; index < 2; index++ {
			var frame map[string]interface{}
			if err := connection.ReadJSON(&frame); err != nil {
				return
			}
			frames <- frame
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := client.Connect(ctx); err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer client.Disconnect(context.Background())
	if err := client.Subscribe("searches", "searches"); err != nil {
		t.Fatalf("subscribe failed: %v", err)
	}
	if err := client.Unsubscribe("missing", "searches", "searches"); err != nil {
		t.Fatalf("unsubscribe failed: %v", err)
	}

	subscribe := <-frames
	unsubscribe := <-frames
	if subscribe["type"] != "subscribe" || unsubscribe["type"] != "unsubscribe" {
		t.Fatalf("unexpected frames: %#v %#v", subscribe, unsubscribe)
	}
	for _, frame := range []map[string]interface{}{subscribe, unsubscribe} {
		topics := frame["data"].(map[string]interface{})["topics"].([]interface{})
		if len(topics) != 1 || topics[0] != "searches" {
			t.Fatalf("unexpected topics: %#v", topics)
		}
	}
	if len(client.GetSubscribedTopics()) != 0 {
		t.Fatalf("local topics were not removed: %v", client.GetSubscribedTopics())
	}
}

func TestWebSocketRestoresSubscriptionsOnConnect(t *testing.T) {
	frames := make(chan map[string]interface{}, 1)
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err != nil {
			return
		}
		defer connection.Close()
		var frame map[string]interface{}
		if err := connection.ReadJSON(&frame); err == nil {
			frames <- frame
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	if err := client.Subscribe("searches"); err != nil {
		t.Fatalf("subscribe before connect failed: %v", err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := client.Connect(ctx); err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer client.Disconnect(context.Background())

	frame := <-frames
	if frame["type"] != "subscribe" {
		t.Fatalf("unexpected frame: %#v", frame)
	}
	topics := frame["data"].(map[string]interface{})["topics"].([]interface{})
	if len(topics) != 1 || topics[0] != "searches" {
		t.Fatalf("unexpected restored topics: %#v", topics)
	}
}

func TestWebSocketReconnectsAfterUnexpectedClose(t *testing.T) {
	var connectionMu sync.Mutex
	connectionCount := 0
	releaseSecond := make(chan struct{})
	defer close(releaseSecond)
	secondConnection := make(chan struct{})
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err != nil {
			return
		}
		connectionMu.Lock()
		connectionCount++
		current := connectionCount
		connectionMu.Unlock()
		if current == 2 {
			close(secondConnection)
		}
		if current == 1 {
			_ = connection.Close()
			return
		}
		defer connection.Close()
		<-releaseSecond
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	client.reconnectDelay = 5 * time.Millisecond
	client.maxReconnectAttempts = 1
	states := make(chan bool, 4)
	client.OnConnectionChange(states)

	if err := client.Connect(context.Background()); err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	waitForState := func(expected bool) {
		t.Helper()
		select {
		case actual := <-states:
			if actual != expected {
				t.Fatalf("expected connection state %v, got %v", expected, actual)
			}
		case <-time.After(time.Second):
			t.Fatalf("timed out waiting for connection state %v", expected)
		}
	}
	waitForState(true)
	waitForState(false)
	waitForState(true)
	select {
	case <-secondConnection:
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for the server to accept the reconnect")
	}

	connectionMu.Lock()
	if connectionCount != 2 {
		t.Fatalf("expected one reconnect, got %d connections", connectionCount)
	}
	connectionMu.Unlock()

	if err := client.Disconnect(context.Background()); err != nil {
		t.Fatalf("disconnect failed: %v", err)
	}
}

func TestWebSocketRejectsInvalidBaseURLBeforeDial(t *testing.T) {
	for _, baseURL := range []string{"ftp://example.test", "example.test"} {
		client := NewClient(baseURL, "token").NewWebSocketClient(false)
		err := client.Connect(context.Background())
		if err == nil || !strings.Contains(err.Error(), "absolute HTTP or HTTPS") {
			t.Fatalf("expected URL validation error for %q, got %v", baseURL, err)
		}
	}
}

func TestWebSocketURLPreservesBasePathAndDropsHTTPQuery(t *testing.T) {
	got, err := websocketURL("https://example.test/slskr/?debug=true#fragment")
	if err != nil {
		t.Fatalf("build WebSocket URL: %v", err)
	}
	if got != "wss://example.test/slskr/api/events/ws" {
		t.Fatalf("unexpected WebSocket URL: %q", got)
	}
}

func TestWebSocketSerializesConcurrentSubscriptionWrites(t *testing.T) {
	const topicCount = 64
	frames := make(chan struct{}, topicCount)
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err != nil {
			return
		}
		defer connection.Close()
		for index := 0; index < topicCount; index++ {
			var frame map[string]interface{}
			if err := connection.ReadJSON(&frame); err != nil {
				return
			}
			frames <- struct{}{}
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := client.Connect(ctx); err != nil {
		t.Fatalf("connect failed: %v", err)
	}
	defer client.Disconnect(context.Background())

	errors := make(chan error, topicCount)
	var writers sync.WaitGroup
	for index := 0; index < topicCount; index++ {
		writers.Add(1)
		go func(topic string) {
			defer writers.Done()
			if err := client.Subscribe(topic); err != nil {
				errors <- err
			}
		}(fmt.Sprintf("topic-%d", index))
	}
	writers.Wait()
	close(errors)
	for err := range errors {
		t.Fatalf("concurrent subscribe failed: %v", err)
	}

	for index := 0; index < topicCount; index++ {
		select {
		case <-frames:
		case <-ctx.Done():
			t.Fatalf("received only %d of %d subscription frames", index, topicCount)
		}
	}
}

func TestWebSocketRollsBackFailedSubscriptionTransitions(t *testing.T) {
	client := NewClient("http://example.test", "token").NewWebSocketClient(false)
	client.connected = true

	if err := client.Subscribe("searches"); err == nil || !strings.Contains(err.Error(), "not connected") {
		t.Fatalf("expected failed subscribe write, got %v", err)
	}
	if topics := client.GetSubscribedTopics(); len(topics) != 0 {
		t.Fatalf("failed subscribe changed local topics: %v", topics)
	}

	client.subscribedTopics["transfers"] = true
	if err := client.Unsubscribe("transfers"); err == nil || !strings.Contains(err.Error(), "not connected") {
		t.Fatalf("expected failed unsubscribe write, got %v", err)
	}
	topics := client.GetSubscribedTopics()
	if len(topics) != 1 || topics[0] != "transfers" {
		t.Fatalf("failed unsubscribe changed local topics: %v", topics)
	}
}

func TestWebSocketConnectRejectsConcurrentDial(t *testing.T) {
	requestStarted := make(chan struct{})
	releaseUpgrade := make(chan struct{})
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		close(requestStarted)
		<-releaseUpgrade
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err == nil {
			defer connection.Close()
			_, _, _ = connection.ReadMessage()
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	connected := make(chan error, 1)
	go func() { connected <- client.Connect(ctx) }()
	<-requestStarted

	if err := client.Connect(ctx); err == nil || !strings.Contains(err.Error(), "in progress") {
		t.Fatalf("expected connection-in-progress error, got %v", err)
	}
	close(releaseUpgrade)
	if err := <-connected; err != nil {
		t.Fatalf("first connection failed: %v", err)
	}
	if err := client.Disconnect(context.Background()); err != nil {
		t.Fatalf("disconnect failed: %v", err)
	}
}

func TestStaleWebSocketReaderCannotClearCurrentConnection(t *testing.T) {
	client := NewClient("http://example.test", "token").NewWebSocketClient(false)
	stale := &websocket.Conn{}
	current := &websocket.Conn{}
	client.connected = true
	client.conn = current

	if client.clearConnectionIfCurrent(stale) || !client.IsConnected() || client.conn != current {
		t.Fatal("stale reader cleared the current connection")
	}
}

func TestDisconnectCancelsInFlightWebSocketConnection(t *testing.T) {
	requestStarted := make(chan struct{})
	releaseUpgrade := make(chan struct{})
	upgrader := websocket.Upgrader{}
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		close(requestStarted)
		<-releaseUpgrade
		connection, err := upgrader.Upgrade(writer, request, nil)
		if err == nil {
			defer connection.Close()
			_, _, _ = connection.ReadMessage()
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token").NewWebSocketClient(false)
	connected := make(chan error, 1)
	go func() { connected <- client.Connect(context.Background()) }()
	<-requestStarted

	if err := client.Disconnect(context.Background()); err != nil {
		t.Fatalf("disconnect during dial failed: %v", err)
	}
	var connectErr error
	select {
	case connectErr = <-connected:
		close(releaseUpgrade)
	case <-time.After(time.Second):
		close(releaseUpgrade)
		t.Fatal("disconnect did not cancel the in-flight connection promptly")
	}
	if connectErr == nil {
		t.Fatal("expected the in-flight connection to fail after disconnect")
	}
	errText := strings.ToLower(connectErr.Error())
	if !strings.Contains(errText, "canceled") && !strings.Contains(errText, "closed") {
		t.Fatalf("expected cancellation or close error, got %v", connectErr)
	}
	if client.IsConnected() {
		t.Fatal("connection became active after disconnect")
	}
}

func TestWebSocketConnectUsesClientTimeout(t *testing.T) {
	requestStarted := make(chan struct{})
	releaseUpgrade := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		close(requestStarted)
		<-releaseUpgrade
		_, _ = writer.Write([]byte("upgrade deliberately delayed"))
	}))
	defer server.Close()

	client := NewClient(server.URL, "token")
	client.Timeout = 10 * time.Millisecond
	websocketClient := client.NewWebSocketClient(false)

	err := websocketClient.Connect(context.Background())
	close(releaseUpgrade)
	if err == nil || !strings.Contains(err.Error(), "timeout") {
		t.Fatalf("expected configured WebSocket deadline, got %v", err)
	}
	select {
	case <-requestStarted:
	default:
		t.Fatal("WebSocket dial did not reach the test server")
	}
}

func TestWebSocketClosedListenerChannelsDoNotPanic(t *testing.T) {
	client := NewClient("http://example.test", "token").NewWebSocketClient(false)
	eventChannel := make(chan interface{}, 1)
	connectionChannel := make(chan bool, 1)
	errorChannel := make(chan error, 1)
	client.On("search.completed", eventChannel)
	client.OnConnectionChange(connectionChannel)
	client.OnError(errorChannel)
	close(eventChannel)
	close(connectionChannel)
	close(errorChannel)

	client.processMessage(map[string]interface{}{"type": "search.completed"})
	client.notifyConnectionListeners(true)
	client.notifyErrorListeners(fmt.Errorf("listener test"))
}

func TestWebSocketListenerUnsubscribeHandles(t *testing.T) {
	client := NewClient("http://example.test", "token").NewWebSocketClient(false)
	eventChannel := make(chan interface{}, 1)
	connectionChannel := make(chan bool, 1)
	errorChannel := make(chan error, 1)

	unlistenEvent := client.On("search.completed", eventChannel)
	unlistenConnection := client.OnConnectionChange(connectionChannel)
	unlistenError := client.OnError(errorChannel)

	client.processMessage(map[string]interface{}{"type": "search.completed"})
	client.notifyConnectionListeners(true)
	client.notifyErrorListeners(fmt.Errorf("listener test"))
	if len(eventChannel) != 1 || len(connectionChannel) != 1 || len(errorChannel) != 1 {
		t.Fatal("registered listeners did not receive notifications")
	}
	for len(eventChannel) > 0 {
		<-eventChannel
	}
	for len(connectionChannel) > 0 {
		<-connectionChannel
	}
	for len(errorChannel) > 0 {
		<-errorChannel
	}

	unlistenEvent()
	unlistenEvent()
	unlistenConnection()
	unlistenError()
	if len(client.eventChannels["search.completed"]) != 0 || len(client.connectionCh) != 0 || len(client.errorCh) != 0 {
		t.Fatal("unsubscribed listeners were retained")
	}

	close(eventChannel)
	close(connectionChannel)
	close(errorChannel)
	client.processMessage(map[string]interface{}{"type": "search.completed"})
	client.notifyConnectionListeners(false)
	client.notifyErrorListeners(fmt.Errorf("listener test"))
}

func startRF042Fixture(t *testing.T) string {
	t.Helper()
	_, testFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("failed to locate Go test source")
	}
	repoRoot := filepath.Dir(filepath.Dir(testFile))
	script := filepath.Join(repoRoot, "web", "e2e", "fixtures", "live-subscription-fixture.py")
	command := exec.Command("python3", script, "--port", "0")
	stdout, err := command.StdoutPipe()
	if err != nil {
		t.Fatalf("create fixture stdout pipe: %v", err)
	}
	var stderr bytes.Buffer
	command.Stderr = &stderr
	if err := command.Start(); err != nil {
		t.Fatalf("start RF-042 fixture: %v", err)
	}
	t.Cleanup(func() {
		if command.Process != nil {
			_ = command.Process.Kill()
		}
		_ = command.Wait()
	})

	line := make(chan string, 1)
	readErr := make(chan error, 1)
	go func() {
		value, readError := bufio.NewReader(stdout).ReadString('\n')
		if readError != nil {
			readErr <- readError
			return
		}
		line <- strings.TrimSpace(value)
	}()
	select {
	case value := <-line:
		if !strings.HasPrefix(value, "PORT=") {
			t.Fatalf("RF-042 fixture did not report a port: %q (stderr: %s)", value, stderr.String())
		}
		return "http://127.0.0.1:" + strings.TrimPrefix(value, "PORT=")
	case err := <-readErr:
		t.Fatalf("RF-042 fixture did not start: %v (stderr: %s)", err, stderr.String())
	case <-time.After(3 * time.Second):
		t.Fatalf("timed out waiting for RF-042 fixture (stderr: %s)", stderr.String())
	}
	return ""
}

func receiveRF042Event(t *testing.T, events <-chan interface{}, expectedID string) map[string]interface{} {
	t.Helper()
	select {
	case raw := <-events:
		message, ok := raw.(map[string]interface{})
		if !ok {
			t.Fatalf("unexpected event value: %#v", raw)
		}
		if message["id"] != expectedID {
			t.Fatalf("expected event %q, got %#v", expectedID, message)
		}
		return message
	case <-time.After(3 * time.Second):
		t.Fatalf("timed out waiting for RF-042 event %q", expectedID)
	}
	return nil
}

func TestWebSocketRF042LiveSubscriptionContract(t *testing.T) {
	baseURL := startRF042Fixture(t)
	client := NewClient(baseURL, "rf042-token").NewWebSocketClient(false)
	client.reconnectDelay = 5 * time.Millisecond
	client.maxReconnectAttempts = 1

	searchEvents := make(chan interface{}, 8)
	transferEvents := make(chan interface{}, 8)
	client.On("search.completed", searchEvents)
	client.On("transfer.completed", transferEvents)

	if err := client.Subscribe("search.completed", "transfer.completed"); err != nil {
		t.Fatalf("subscribe before connect failed: %v", err)
	}
	if err := client.Connect(context.Background()); err != nil {
		t.Fatalf("connect to RF-042 fixture failed: %v", err)
	}

	receiveRF042Event(t, searchEvents, "rf042-initial")
	if err := client.Unsubscribe("transfer.completed"); err != nil {
		t.Fatalf("unsubscribe failed: %v", err)
	}
	receiveRF042Event(t, searchEvents, "rf042-after-unsubscribe")
	receiveRF042Event(t, searchEvents, "rf042-reconnect")

	select {
	case event := <-transferEvents:
		t.Fatalf("filtered transfer event was delivered: %#v", event)
	default:
	}
	topics := client.GetSubscribedTopics()
	if len(topics) != 1 || topics[0] != "search.completed" {
		t.Fatalf("unexpected topics after unsubscribe/reconnect: %v", topics)
	}
	if err := client.Disconnect(context.Background()); err != nil {
		t.Fatalf("disconnect failed: %v", err)
	}
}
