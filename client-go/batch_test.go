package slskr

import "testing"

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
