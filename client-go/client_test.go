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
		case request.Method == http.MethodPost && request.URL.Path == "/api/searches":
			var payload map[string]interface{}
			if err := json.NewDecoder(request.Body).Decode(&payload); err != nil {
				t.Errorf("decode search payload: %v", err)
			}
			if payload["query"] != "ambient" {
				t.Errorf("search payload used the wrong query: %#v", payload)
			}
			if payload["room"] == "lounge" && payload["target"] != "room" {
				t.Errorf("targeted search payload used the wrong target: %#v", payload)
			}
			write(`{"searchId":"search-123","query":"ambient","results":[]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/searches/search-123":
			if request.URL.Query().Get("limit") != "10" || request.URL.Query().Get("offset") != "2" {
				t.Errorf("search details pagination was not encoded: %q", request.URL.RawQuery)
			}
			write(`{"id":"search-123","query":"ambient","results":[]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/messages":
			write(`{"entries":[{"id":1}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/messages/alice":
			if request.URL.Query().Get("limit") != "10" || request.URL.Query().Get("offset") != "2" {
				t.Errorf("user message pagination was not encoded: %q", request.URL.RawQuery)
			}
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
		case request.Method == http.MethodPost && request.URL.Path == "/api/transfers":
			var payload map[string]interface{}
			if err := json.NewDecoder(request.Body).Decode(&payload); err != nil {
				t.Errorf("decode transfer payload: %v", err)
			}
			if payload["peer_username"] == "alice" && (payload["direction"] != float64(0) || payload["filename"] != "track.flac") {
				t.Errorf("download transfer payload used the wrong wire fields: %#v", payload)
			}
			if payload["peer_username"] == "bob" && (payload["direction"] != float64(1) || payload["filename"] != "upload.flac") {
				t.Errorf("upload transfer payload used the wrong wire fields: %#v", payload)
			}
			if (payload["peer_username"] != "alice" && payload["peer_username"] != "bob") ||
				(payload["filename"] != "track.flac" && payload["filename"] != "upload.flac") {
				t.Errorf("transfer payload used the wrong wire fields: %#v", payload)
			}
			if payload["peer_username"] == "bob" {
				write(`{"id":6,"status":"queued"}`)
			} else {
				write(`{"id":5,"status":"queued"}`)
			}
		case request.Method == http.MethodGet && request.URL.Path == "/api/transfers/5":
			write(`{"id":5,"status":"queued"}`)
		case request.Method == http.MethodDelete && request.URL.Path == "/api/transfers/5":
			writer.WriteHeader(http.StatusNoContent)
		case request.Method == http.MethodGet && request.URL.Path == "/api/users":
			write(`{"entries":[{"username":"alice"}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/rooms":
			if request.URL.Query().Get("limit") != "10" || request.URL.Query().Get("offset") != "2" {
				t.Errorf("room pagination was not encoded: %q", request.URL.RawQuery)
			}
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
	search, err := client.CreateSearch(ctx, "ambient")
	if err != nil || search["id"] != "search-123" || search["searchId"] != "search-123" {
		t.Fatalf("unexpected create search response: %#v, %v", search, err)
	}
	if _, err := client.CreateSearchWithOptions(ctx, "ambient", SearchOptions{Room: "lounge", Target: "room"}); err != nil {
		t.Fatalf("targeted search failed: %v", err)
	}
	searchDetails, err := client.GetSearchDetails(ctx, "search-123", 10, 2)
	if err != nil || searchDetails["id"] != "search-123" {
		t.Fatalf("unexpected search details response: %#v, %v", searchDetails, err)
	}
	messages, err := client.ListMessages(ctx, 10, 0)
	if err != nil || len(messages) != 1 {
		t.Fatalf("unexpected messages response: %#v, %v", messages, err)
	}
	userMessages, err := client.GetUserMessages(ctx, "alice", 10, 2)
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
	createdTransfer, err := client.CreateTransfer(ctx, "download", "alice", "track.flac")
	if err != nil || createdTransfer["id"] != float64(5) {
		t.Fatalf("unexpected created transfer response: %#v, %v", createdTransfer, err)
	}
	uploadTransfer, err := client.CreateTransfer(ctx, "upload", "bob", "upload.flac")
	if err != nil || uploadTransfer["id"] != float64(6) {
		t.Fatalf("unexpected created upload response: %#v, %v", uploadTransfer, err)
	}
	transfer, err := client.GetTransfer(ctx, "5")
	if err != nil || transfer["id"] != float64(5) {
		t.Fatalf("unexpected transfer details response: %#v, %v", transfer, err)
	}
	if err := client.CancelTransfer(ctx, "5"); err != nil {
		t.Fatalf("cancel transfer failed: %v", err)
	}
	users, err := client.ListUsers(ctx, 10, 0)
	if err != nil || len(users) != 1 {
		t.Fatalf("unexpected users response: %#v, %v", users, err)
	}
	rooms, err := client.ListRooms(ctx, 10, 2)
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

func TestClientCoversSessionBrowseEventAndCacheRoutes(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		writer.Header().Set("Content-Type", "application/json")
		write := func(body string) { _, _ = writer.Write([]byte(body)) }
		switch {
		case request.Method == http.MethodGet && request.URL.Path == "/api/session":
			write(`{"state":"connected","username":"alice","privileges_seconds":60}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/session/connect":
			write(`{"accepted":true}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/session/ping":
			write(`{"accepted":true}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/session/disconnect":
			write(`{"accepted":true}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/session/privileges/check":
			write(`{"accepted":true}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/messages/7/ack":
			writer.WriteHeader(http.StatusNoContent)
		case request.Method == http.MethodGet && request.URL.EscapedPath() == "/api/users/alice/info":
			write(`{"username":"alice"}`)
		case request.Method == http.MethodGet && request.URL.EscapedPath() == "/api/users/alice/browse":
			if request.URL.Query().Get("folder") != "Albums" {
				t.Errorf("browse folder was not encoded: %q", request.URL.RawQuery)
			}
			write(`{"entries":[{"filename":"track.flac"}]}`)
		case request.Method == http.MethodPost && request.URL.EscapedPath() == "/api/users/alice/browse/request":
			write(`{"username":"alice","status":"pending"}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/browse/requests":
			if request.URL.Query().Get("status") != "pending" {
				t.Errorf("browse status was not encoded: %q", request.URL.RawQuery)
			}
			write(`{"requests":[{"username":"alice","status":"pending"}]}`)
		case request.Method == http.MethodPost && request.URL.EscapedPath() == "/api/users/alice/browse/folder":
			write(`{"entries":[]}`)
		case request.Method == http.MethodPost && request.URL.EscapedPath() == "/api/users/alice/browse/cancel":
			write(`{"cancelled":true}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/events":
			if request.URL.Query().Get("kind") != "transfer.completed" {
				t.Errorf("event kind was not encoded: %q", request.URL.RawQuery)
			}
			if topic := request.URL.Query().Get("topic"); topic != "" && topic != "searches" {
				t.Errorf("event topic was not encoded: %q", request.URL.RawQuery)
			}
			if query := request.URL.Query().Get("q"); query != "" && query != "ambient & live" {
				t.Errorf("event query was not encoded: %q", request.URL.RawQuery)
			}
			if request.URL.Query().Get("topic") == "searches" &&
				request.URL.Query().Get("q") == "ambient & live" &&
				(request.URL.Query().Get("limit") != "10" || request.URL.Query().Get("offset") != "20") {
				t.Errorf("filtered event pagination was not encoded: %q", request.URL.RawQuery)
			}
			write(`{"events":[{"type":"transfer.completed"}]}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/mediacore/retrieve/stats":
			write(`{"entries":4}`)
		case request.Method == http.MethodPost && request.URL.Path == "/api/mediacore/retrieve/cache/clear":
			write(`{"cleared":2}`)
		case request.Method == http.MethodGet && request.URL.Path == "/api/health":
			write(`{"status":"ok"}`)
		default:
			t.Errorf("unexpected %s %s", request.Method, request.URL.RequestURI())
			writer.WriteHeader(http.StatusNotFound)
		}
	}))
	defer server.Close()

	client := NewClient(server.URL, "token")
	ctx := context.Background()

	sessions, err := client.GetSessions(ctx)
	if err != nil || len(sessions) != 1 || sessions[0]["username"] != "alice" {
		t.Fatalf("unexpected sessions response: %#v, %v", sessions, err)
	}
	if _, err := client.CreateSession(ctx, "server", nil); err != nil {
		t.Fatalf("create session failed: %v", err)
	}
	if _, err := client.PingSession(ctx, "server"); err != nil {
		t.Fatalf("ping session failed: %v", err)
	}
	if err := client.DisconnectSession(ctx, "server"); err != nil {
		t.Fatalf("disconnect session failed: %v", err)
	}
	privileges, err := client.GetSessionPrivileges(ctx, "server")
	if err != nil || privileges["user_id"] != "alice" {
		t.Fatalf("unexpected privileges response: %#v, %v", privileges, err)
	}
	if err := client.AcknowledgeMessage(ctx, "7"); err != nil {
		t.Fatalf("acknowledge message failed: %v", err)
	}
	if _, err := client.GetUser(ctx, "alice"); err != nil {
		t.Fatalf("get user failed: %v", err)
	}
	if _, err := client.BrowseUser(ctx, "alice", "Albums", 10, 0); err != nil {
		t.Fatalf("browse user failed: %v", err)
	}
	if _, err := client.RequestBrowse(ctx, "alice"); err != nil {
		t.Fatalf("request browse failed: %v", err)
	}
	if _, err := client.RequestBrowse(ctx, "alice", "Albums"); err != nil {
		t.Fatalf("folder browse request failed: %v", err)
	}
	if requests, err := client.GetBrowseRequests(ctx, "pending", 10, 0); err != nil || len(requests) != 1 {
		t.Fatalf("unexpected browse requests response: %#v, %v", requests, err)
	}
	if _, err := client.RespondToBrowseRequest(ctx, "alice", "accept", "Albums"); err != nil {
		t.Fatalf("accept browse request failed: %v", err)
	}
	if _, err := client.RespondToBrowseRequest(ctx, "alice", "reject", ""); err != nil {
		t.Fatalf("reject browse request failed: %v", err)
	}
	if events, err := client.GetEvents(ctx, "transfer.completed", 10, 0); err != nil || len(events) != 1 {
		t.Fatalf("unexpected events response: %#v, %v", events, err)
	}
	if events, err := client.GetEventsWithFilters(ctx, "transfer.completed", "searches", "ambient & live", 10, 20); err != nil || len(events) != 1 {
		t.Fatalf("unexpected filtered events response: %#v, %v", events, err)
	}
	if _, err := client.GetCacheStats(ctx); err != nil {
		t.Fatalf("get cache stats failed: %v", err)
	}
	if _, err := client.InvalidateCache(ctx, []string{"artist:alice"}); err != nil {
		t.Fatalf("invalidate cache failed: %v", err)
	}
}

func TestClientHandlesNilHTTPClient(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		_, _ = writer.Write([]byte(`{"status":"ok"}`))
	}))
	defer server.Close()

	client := NewClient(server.URL, "token")
	client.HTTPClient = nil
	if _, err := client.Health(context.Background()); err != nil {
		t.Fatalf("nil HTTP client should use the default transport: %v", err)
	}
}
