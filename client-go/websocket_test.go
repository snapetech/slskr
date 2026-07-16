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
