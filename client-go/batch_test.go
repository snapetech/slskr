package slskr

import (
	"context"
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
	first[0].Body["query"] = "mutated snapshot"
	first[0].Body["options"].(map[string]interface{})["filters"].([]interface{})[0] = "mutated snapshot"

	stored := builder.GetOperations()[0].Body
	if stored["query"] != "ambient" {
		t.Fatalf("builder retained aliased query: %v", stored["query"])
	}
	storedFilters := stored["options"].(map[string]interface{})["filters"].([]interface{})
	if storedFilters[0] != "lossless" {
		t.Fatalf("builder retained aliased filters: %v", storedFilters)
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

func stringPointer(value string) *string {
	return &value
}
