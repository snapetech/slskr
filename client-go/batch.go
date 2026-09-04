package slskr

import (
	"context"
	"fmt"
	"reflect"
)

// BatchOperation represents a single operation in a batch
type BatchOperation struct {
	ID     string      `json:"id"`
	Method string      `json:"method"`
	Path   string      `json:"path"`
	Body   interface{} `json:"body,omitempty"`
}

// BatchResult represents the result of a batch operation
type BatchResult struct {
	ID     string      `json:"id"`
	Status int         `json:"status"`
	Body   interface{} `json:"body"`
}

// BatchResponse represents the response from batch operations
type BatchResponse struct {
	Results     []BatchResult `json:"results"`
	TotalTimeMs int           `json:"total_time_ms"`
}

// IsSuccess checks if operation was successful
func (b *BatchResult) IsSuccess() bool {
	return b.Status >= 200 && b.Status < 300
}

// IsError checks if operation failed
func (b *BatchResult) IsError() bool {
	return !b.IsSuccess()
}

// BatchBuilder helps build batch operations
type BatchBuilder struct {
	client     *Client
	operations []BatchOperation
	opCounter  int
}

// NewBatchBuilder creates a new batch builder
func (c *Client) NewBatchBuilder() *BatchBuilder {
	return &BatchBuilder{
		client:     c,
		operations: []BatchOperation{},
	}
}

// Get adds a GET operation
func (b *BatchBuilder) Get(path string, opID *string) *BatchBuilder {
	id := b.operationID(opID)
	b.operations = append(b.operations, BatchOperation{
		ID:     id,
		Method: "GET",
		Path:   path,
	})
	return b
}

// Post adds a POST operation. The body may be any JSON value accepted by the
// batch API, including objects, arrays, strings, numbers, booleans, and null.
func (b *BatchBuilder) Post(path string, body interface{}, opID *string) *BatchBuilder {
	id := b.operationID(opID)
	b.operations = append(b.operations, BatchOperation{
		ID:     id,
		Method: "POST",
		Path:   path,
		Body:   cloneJSONValue(body),
	})
	return b
}

// Put adds a PUT operation. The body may be any JSON value accepted by the
// batch API, including objects, arrays, strings, numbers, booleans, and null.
func (b *BatchBuilder) Put(path string, body interface{}, opID *string) *BatchBuilder {
	id := b.operationID(opID)
	b.operations = append(b.operations, BatchOperation{
		ID:     id,
		Method: "PUT",
		Path:   path,
		Body:   cloneJSONValue(body),
	})
	return b
}

// Delete adds a DELETE operation
func (b *BatchBuilder) Delete(path string, opID *string) *BatchBuilder {
	id := b.operationID(opID)
	b.operations = append(b.operations, BatchOperation{
		ID:     id,
		Method: "DELETE",
		Path:   path,
	})
	return b
}

// Size returns number of operations
func (b *BatchBuilder) Size() int {
	return len(b.operations)
}

func (b *BatchBuilder) operationID(opID *string) string {
	if opID != nil {
		b.opCounter++
		return *opID
	}

	for {
		id := fmt.Sprintf("op-%d", b.opCounter)
		b.opCounter++
		duplicate := false
		for _, operation := range b.operations {
			if operation.ID == id {
				duplicate = true
				break
			}
		}
		if !duplicate {
			return id
		}
	}
}

// Clear clears all operations
func (b *BatchBuilder) Clear() *BatchBuilder {
	b.operations = []BatchOperation{}
	b.opCounter = 0
	return b
}

// GetOperations returns copy of operations
func (b *BatchBuilder) GetOperations() []BatchOperation {
	ops := make([]BatchOperation, len(b.operations))
	for index, operation := range b.operations {
		operation.Body = cloneJSONValue(operation.Body)
		ops[index] = operation
	}
	return ops
}

func cloneJSONValue(value interface{}) interface{} {
	cloned := cloneJSONReflectValue(reflect.ValueOf(value))
	if !cloned.IsValid() {
		return nil
	}
	return cloned.Interface()
}

func cloneJSONReflectValue(value reflect.Value) reflect.Value {
	if !value.IsValid() {
		return value
	}

	switch value.Kind() {
	case reflect.Interface:
		if value.IsNil() {
			return reflect.Zero(value.Type())
		}
		cloned := cloneJSONReflectValue(value.Elem())
		result := reflect.New(value.Type()).Elem()
		result.Set(cloned)
		return result
	case reflect.Map:
		if value.IsNil() {
			return reflect.Zero(value.Type())
		}
		result := reflect.MakeMapWithSize(value.Type(), value.Len())
		iter := value.MapRange()
		for iter.Next() {
			result.SetMapIndex(iter.Key(), cloneJSONReflectValue(iter.Value()))
		}
		return result
	case reflect.Slice:
		if value.IsNil() {
			return reflect.Zero(value.Type())
		}
		result := reflect.MakeSlice(value.Type(), value.Len(), value.Len())
		for index := 0; index < value.Len(); index++ {
			result.Index(index).Set(cloneJSONReflectValue(value.Index(index)))
		}
		return result
	case reflect.Array:
		result := reflect.New(value.Type()).Elem()
		for index := 0; index < value.Len(); index++ {
			result.Index(index).Set(cloneJSONReflectValue(value.Index(index)))
		}
		return result
	case reflect.Pointer:
		if value.IsNil() {
			return reflect.Zero(value.Type())
		}
		result := reflect.New(value.Type().Elem())
		result.Elem().Set(cloneJSONReflectValue(value.Elem()))
		return result
	default:
		return value
	}
}

// Execute executes the batch operations
func (b *BatchBuilder) Execute(ctx context.Context) (*BatchResponse, error) {
	if len(b.operations) == 0 {
		return nil, fmt.Errorf("batch is empty")
	}

	if len(b.operations) > 100 {
		return nil, fmt.Errorf("batch cannot exceed 100 operations")
	}
	if err := validateBatchOperationIDs(b.operations); err != nil {
		return nil, err
	}

	request := map[string]interface{}{
		"operations": b.operations,
	}

	result, err := b.client.post(ctx, "/api/batch", request, true)
	if err != nil {
		return nil, err
	}

	var response BatchResponse
	response.TotalTimeMs = int(getFloat64(result, "total_time_ms"))

	if results, ok := result["results"].([]interface{}); ok {
		for _, r := range results {
			if rm, ok := r.(map[string]interface{}); ok {
				br := BatchResult{
					ID:     getString(rm, "id"),
					Status: int(getFloat64(rm, "status")),
					Body:   rm["body"],
				}
				response.Results = append(response.Results, br)
			}
		}
	}

	return &response, nil
}

func validateBatchOperationIDs(operations []BatchOperation) error {
	seen := make(map[string]struct{}, len(operations))
	for _, operation := range operations {
		if _, exists := seen[operation.ID]; exists {
			return fmt.Errorf("batch contains duplicate operation ID %q", operation.ID)
		}
		seen[operation.ID] = struct{}{}
	}
	return nil
}

// AllSuccessful checks if all operations succeeded
func (br *BatchResponse) AllSuccessful() bool {
	for _, r := range br.Results {
		if !r.IsSuccess() {
			return false
		}
	}
	return true
}

// GetSuccessful returns only successful operations
func (br *BatchResponse) GetSuccessful() []BatchResult {
	var successful []BatchResult
	for _, r := range br.Results {
		if r.IsSuccess() {
			successful = append(successful, r)
		}
	}
	return successful
}

// GetFailed returns only failed operations
func (br *BatchResponse) GetFailed() []BatchResult {
	var failed []BatchResult
	for _, r := range br.Results {
		if r.IsError() {
			failed = append(failed, r)
		}
	}
	return failed
}

// Helper functions for type conversion
func getString(m map[string]interface{}, key string) string {
	if v, ok := m[key]; ok {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}

func getFloat64(m map[string]interface{}, key string) float64 {
	if v, ok := m[key]; ok {
		if f, ok := v.(float64); ok {
			return f
		}
	}
	return 0
}
