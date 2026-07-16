package slskr

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

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
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	connected := make(chan error, 1)
	go func() { connected <- client.Connect(ctx) }()
	<-requestStarted

	if err := client.Disconnect(context.Background()); err != nil {
		t.Fatalf("disconnect during dial failed: %v", err)
	}
	close(releaseUpgrade)
	if err := <-connected; err == nil || !strings.Contains(err.Error(), "canceled") {
		t.Fatalf("expected canceled connection, got %v", err)
	}
	if client.IsConnected() {
		t.Fatal("connection became active after disconnect")
	}
}
