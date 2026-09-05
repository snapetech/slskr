package slskr

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestBatchBuilderOwnsNestedRequestBodies(t *testing.T) {
	client := NewClient("http://example.test", "token")
	filters := []interface{}{"lossless"}
	body := map[string]interface{}{
		"query":   "ambient",
		"options": map[string]interface{}{"filters": filters},
	}
	builder := client.NewBatchBuilder().Post("/api/searches", body, nil)

	filters[0] = "mutated input"
	body["query"] = "mutated input"
	first := builder.GetOperations()
	firstBody := first[0].Body.(map[string]interface{})
	firstBody["query"] = "mutated snapshot"
	firstBody["options"].(map[string]interface{})["filters"].([]interface{})[0] = "mutated snapshot"

	stored := builder.GetOperations()[0].Body.(map[string]interface{})
	if stored["query"] != "ambient" {
		t.Fatalf("builder retained aliased query: %v", stored["query"])
	}
	storedFilters := stored["options"].(map[string]interface{})["filters"].([]interface{})
	if storedFilters[0] != "lossless" {
		t.Fatalf("builder retained aliased filters: %v", storedFilters)
	}
}

func TestBatchBuilderSupportsAnyJSONBody(t *testing.T) {
	client := NewClient("http://example.test", "token")
	values := []string{"lossless", "320kbps"}
	builder := client.NewBatchBuilder().Post("/api/searches", values, nil).Put("/api/config", "compact", nil)

	values[0] = "mutated input"
	operations := builder.GetOperations()
	storedValues, ok := operations[0].Body.([]string)
	if !ok {
		t.Fatalf("expected array body, got %T", operations[0].Body)
	}
	if storedValues[0] != "lossless" {
		t.Fatalf("builder retained aliased array body: %v", storedValues)
	}
	if operations[1].Body != "compact" {
		t.Fatalf("expected scalar body, got %#v", operations[1].Body)
	}

	wire, err := json.Marshal(map[string]interface{}{"operations": operations})
	if err != nil {
		t.Fatalf("marshal batch operations: %v", err)
	}
	if !strings.Contains(string(wire), `"body":["lossless"`) || !strings.Contains(string(wire), `"body":"compact"`) {
		t.Fatalf("batch body shapes were not preserved on the wire: %s", wire)
	}
}

func TestBatchResultTreatsRedirectAsError(t *testing.T) {
	result := BatchResult{ID: "redirect", Status: 302}
	if !result.IsError() {
		t.Fatal("expected a redirect result to be treated as an error")
	}
	if result.IsSuccess() {
		t.Fatal("expected a redirect result not to be treated as successful")
	}
}

func TestBatchBuilderAvoidsGeneratedIDCollisions(t *testing.T) {
	client := NewClient("http://example.test", "token")
	builder := client.NewBatchBuilder()
	builder.Get("/api/health", stringPointer("op-0"))
	builder.Get("/api/version", nil)

	operations := builder.GetOperations()
	if operations[1].ID == operations[0].ID {
		t.Fatalf("generated operation ID collided with explicit ID: %#v", operations)
	}
}

func TestBatchBuilderRejectsDuplicateIDsBeforeSending(t *testing.T) {
	client := NewClient("http://example.test", "token")
	builder := client.NewBatchBuilder()
	builder.Get("/api/health", stringPointer("same"))
	builder.Get("/api/version", stringPointer("same"))

	if _, err := builder.Execute(context.Background()); err == nil || !strings.Contains(err.Error(), "duplicate operation ID") {
		t.Fatalf("expected duplicate operation ID error, got %v", err)
	}
}

func TestBatchExecuteRejectsMalformedResponse(t *testing.T) {
	tests := []struct {
		name string
		body string
	}{
		{
			name: "missing results",
			body: `{"total_time_ms":1}`,
		},
		{
			name: "missing total time",
			body: `{"results":[]}`,
		},
		{
			name: "non-object result",
			body: `{"results":[null],"total_time_ms":1}`,
		},
		{
			name: "missing result id",
			body: `{"results":[{"status":200,"body":null}],"total_time_ms":1}`,
		},
		{
			name: "invalid result status",
			body: `{"results":[{"id":"op-1","status":200.5,"body":null}],"total_time_ms":1}`,
		},
		{
			name: "missing result body",
			body: `{"results":[{"id":"op-1","status":200}],"total_time_ms":1}`,
		},
		{
			name: "negative total time",
			body: `{"results":[],"total_time_ms":-1}`,
		},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
				writer.Header().Set("Content-Type", "application/json")
				_, _ = writer.Write([]byte(test.body))
			}))
			defer server.Close()

			builder := NewClient(server.URL, "token").NewBatchBuilder()
			builder.Get("/api/health", stringPointer("op-1"))
			_, err := builder.Execute(context.Background())
			if err == nil {
				t.Fatal("expected malformed batch response error")
			}
			var contractErr *ResponseContractError
			if !errors.As(err, &contractErr) {
				t.Fatalf("expected ResponseContractError, got %T: %v", err, err)
			}
			if contractErr.Resource != "batch" {
				t.Fatalf("unexpected contract resource: %q", contractErr.Resource)
			}
		})
	}
}

func TestBatchExecuteParsesCompleteResponse(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, _ *http.Request) {
		writer.Header().Set("Content-Type", "application/json")
		_, _ = writer.Write([]byte(`{"results":[{"id":"op-1","status":200,"body":null}],"total_time_ms":4}`))
	}))
	defer server.Close()

	builder := NewClient(server.URL, "token").NewBatchBuilder()
	builder.Get("/api/health", stringPointer("op-1"))
	response, err := builder.Execute(context.Background())
	if err != nil {
		t.Fatalf("execute failed: %v", err)
	}
	if response.TotalTimeMs != 4 || len(response.Results) != 1 {
		t.Fatalf("unexpected batch response: %#v", response)
	}
	if response.Results[0].ID != "op-1" || response.Results[0].Status != http.StatusOK || response.Results[0].Body != nil {
		t.Fatalf("unexpected batch result: %#v", response.Results[0])
	}
}

func stringPointer(value string) *string {
	return &value
}
