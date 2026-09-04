package slskr

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestClientValidatesAndNormalizesRESTBaseURL(t *testing.T) {
	for _, baseURL := range []string{"ftp://example.test", "example.test", "https://user:pass@example.test"} {
		_, err := NewClient(baseURL, "token").Health(context.Background())
		if err == nil || !strings.Contains(err.Error(), "absolute HTTP or HTTPS") {
			t.Fatalf("expected URL validation error for %q, got %v", baseURL, err)
		}
	}

	client := NewClient("https://example.test/slskr/?debug=true#fragment", "token")
	if client.BaseURL != "https://example.test/slskr" {
		t.Fatalf("unexpected normalized base URL: %q", client.BaseURL)
	}
}

func TestClientTimeoutFieldControlsRequests(t *testing.T) {
	requestStarted := make(chan struct{})
	releaseRequest := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		close(requestStarted)
		<-releaseRequest
		writer.Header().Set("Content-Type", "application/json")
		_, _ = writer.Write([]byte(`{"status":"ok"}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "token")
	client.Timeout = 10 * time.Millisecond
	done := make(chan error, 1)
	go func() {
		_, err := client.Health(context.Background())
		done <- err
	}()
	<-requestStarted

	select {
	case err := <-done:
		if err == nil || !errors.Is(err, context.DeadlineExceeded) {
			t.Fatalf("expected configured deadline error, got %v", err)
		}
	case <-time.After(time.Second):
		t.Fatal("request ignored Client.Timeout")
	}
	close(releaseRequest)
}

func TestClientRejectsAuthenticatedCrossOriginRedirects(t *testing.T) {
	receivedAuthorization := make(chan string, 1)
	target := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		receivedAuthorization <- request.Header.Get("Authorization")
		writer.Header().Set("Content-Type", "application/json")
		_, _ = writer.Write([]byte(`{"status":"ok"}`))
	}))
	defer target.Close()

	source := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		http.Redirect(writer, &http.Request{}, target.URL+"/api/health", http.StatusFound)
	}))
	defer source.Close()

	_, err := NewClient(source.URL, "secret-token").GetConfig(context.Background())
	if err == nil || !strings.Contains(err.Error(), "outside configured API origin") {
		t.Fatalf("expected cross-origin redirect rejection, got %v", err)
	}
	select {
	case authorization := <-receivedAuthorization:
		t.Fatalf("redirect target received Authorization header %q", authorization)
	default:
	}
}

func TestClientRejectsOversizedSuccessResponse(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		writer.Header().Set("Content-Length", fmt.Sprint(maxHTTPResponseBytes+1))
		writer.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	_, err := NewClient(server.URL, "token").Health(context.Background())
	if err == nil || !strings.Contains(err.Error(), "exceeds") {
		t.Fatalf("expected oversized response error, got %v", err)
	}
}

func TestClientBoundsChunkedErrorResponse(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		writer.WriteHeader(http.StatusBadGateway)
		_, _ = writer.Write([]byte(strings.Repeat("x", maxHTTPErrorBytes+1)))
	}))
	defer server.Close()

	_, err := NewClient(server.URL, "token").Health(context.Background())
	if err == nil || !strings.Contains(err.Error(), "response body exceeds") {
		t.Fatalf("expected bounded API error, got %v", err)
	}
}

func TestClientRejectsTrailingJSON(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		_, _ = writer.Write([]byte(`{"status":"ok"} {"unexpected":true}`))
	}))
	defer server.Close()

	if _, err := NewClient(server.URL, "token").Health(context.Background()); err == nil {
		t.Fatal("expected trailing JSON to be rejected")
	}
}

func TestClientUsesDaemonWireContracts(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		writer.Header().Set("Content-Type", "application/json")
		write := func(body string) {
			_, _ = writer.Write([]byte(body))
		}

		switch {
		case request.Method == http.MethodGet && request.URL.Path == "/api/searches":
			write(`[{"id":"search-1"}]`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/messages":
			write(`{"entries":[{"id":1}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/messages/alice":
			write(`{"entries":[{"id":2}]}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/messages":
			var payload map[string]interface{}
			if err := json.NewDecoder(request.Body).Decode(&payload); err != nil {
				t.Errorf("decode message payload: %v", err)
			}
			if payload["username"] != "alice" || payload["body"] != "hello" {
				t.Errorf("message payload used the wrong wire fields: %#v", payload)
			}
			write(`{"id":3}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/messages/7/ack":
			writer.WriteHeader(http.StatusNoContent)
		case request.Method == http.MethodGet && request.URL.Path == "/api/transfers":
			if request.URL.Query().Get("direction") != "0" {
				t.Errorf("download direction was not encoded as 0: %q", request.URL.RawQuery)
			}
			write(`{"entries":[{"id":4}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/users":
			write(`{"entries":[{"username":"alice"}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/rooms":
			write(`{"entries":[{"name":"lounge"}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/rooms/lounge":
			write(`{"name":"lounge"}`)
		case request.Method == http.MethodPost && request.URL.EscapedPath() == "/api/rooms/lounge%20room/join":
			var payload map[string]interface{}
			if err := json.NewDecoder(request.Body).Decode(&payload); err != nil {
				t.Errorf("decode join payload: %v", err)
			}
			if payload["name"] != "lounge room" {
				t.Errorf("join payload used the wrong room name: %#v", payload)
			}
			write(`{"name":"lounge room"}`)
		case request.Method == http.MethodDelete && request.URL.EscapedPath() == "/api/rooms/lounge%20room/join":
			writer.WriteHeader(http.StatusNoContent)
		case request.Method == http.MethodGet && request.URL.Path == "/api/shares":
			write(`{"local":[{"path":"/music"}]}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/shares/rescan":
			write(`{"accepted":true}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/config/download-filter":
			write(`{"exclude":["tmp"]}`)
		case request.Method == http.MethodPut && request.URL.Path == "/api/config/download-filter":
			var payload map[string]interface{}
			if err := json.NewDecoder(request.Body).Decode(&payload); err != nil {
				t.Errorf("decode filter payload: %v", err)
			}
			if _, ok := payload["exclude"].([]interface{}); !ok {
				t.Errorf("filter payload used the wrong shape: %#v", payload)
			}
			write(`{"exclude":["private"]}`)
		default:
			t.Errorf("unexpected %s %s", request.Method, request.URL.RequestURI())
			writer.WriteHeader(http.StatusNotFound)
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token")
	ctx := context.Background()

	searches, err := client.ListSearches(ctx, 10, 0)
	if err != nil || len(searches) != 1 || searches[0]["id"] != "search-1" {
		t.Fatalf("unexpected searches response: %#v, %v", searches, err)
	}
	messages, err := client.ListMessages(ctx, 10, 0)
	if err != nil || len(messages) != 1 {
		t.Fatalf("unexpected messages response: %#v, %v", messages, err)
	}
	userMessages, err := client.GetUserMessages(ctx, "alice", 10)
	if err != nil || len(userMessages) != 1 {
		t.Fatalf("unexpected user messages response: %#v, %v", userMessages, err)
	}
	if _, err := client.SendMessage(ctx, "alice", "hello"); err != nil {
		t.Fatalf("send message failed: %v", err)
	}
	if err := client.AcknowledgeMessage(ctx, "7"); err != nil {
		t.Fatalf("acknowledge message failed: %v", err)
	}
	transfers, err := client.ListTransfers(ctx, "download", "", 10, 0)
	if err != nil || len(transfers) != 1 {
		t.Fatalf("unexpected transfers response: %#v, %v", transfers, err)
	}
	users, err := client.ListUsers(ctx, 10, 0)
	if err != nil || len(users) != 1 {
		t.Fatalf("unexpected users response: %#v, %v", users, err)
	}
	rooms, err := client.ListRooms(ctx)
	if err != nil || len(rooms) != 1 {
		t.Fatalf("unexpected rooms response: %#v, %v", rooms, err)
	}
	if _, err := client.GetRoom(ctx, "lounge"); err != nil {
		t.Fatalf("get room failed: %v", err)
	}
	if _, err := client.JoinRoom(ctx, "lounge room"); err != nil {
		t.Fatalf("join room failed: %v", err)
	}
	if err := client.LeaveRoom(ctx, "lounge room"); err != nil {
		t.Fatalf("leave room failed: %v", err)
	}
	shares, err := client.ListShares(ctx, 10, 0)
	if err != nil || len(shares) != 1 {
		t.Fatalf("unexpected shares response: %#v, %v", shares, err)
	}
	if _, err := client.RefreshShares(ctx); err != nil {
		t.Fatalf("refresh shares failed: %v", err)
	}
	filters, err := client.GetFilters(ctx)
	if err != nil || filters["exclude"] == nil {
		t.Fatalf("unexpected filters response: %#v, %v", filters, err)
	}
	if _, err := client.UpdateFilters(ctx, map[string]interface{}{"exclude": []string{"private"}}); err != nil {
		t.Fatalf("update filters failed: %v", err)
	}
}
