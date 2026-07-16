package slskr

import (
	"context"
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
