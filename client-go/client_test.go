package slskr

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
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
