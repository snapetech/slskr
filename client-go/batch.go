package slskr

import (
	"context"
	"fmt"
	"math"
	"reflect"
	"strings"
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

	return parseBatchResponse(result)
}

func parseBatchResponse(result map[string]interface{}) (*BatchResponse, error) {
	rawTotalTime, ok := result["total_time_ms"]
	if !ok {
		return nil, invalidBatchResponse()
	}
	totalTimeMs, ok := responseInteger(rawTotalTime, 0, int64(maxInt()))
	if !ok {
		return nil, invalidBatchResponse()
	}

	rawResults, ok := result["results"].([]interface{})
	if !ok {
		return nil, invalidBatchResponse()
	}

	response := &BatchResponse{
		Results:     make([]BatchResult, 0, len(rawResults)),
		TotalTimeMs: totalTimeMs,
	}
	for _, rawResult := range rawResults {
		resultObject, ok := rawResult.(map[string]interface{})
		if !ok {
			return nil, invalidBatchResponse()
		}

		id, ok := resultObject["id"].(string)
		if !ok || strings.TrimSpace(id) == "" {
			return nil, invalidBatchResponse()
		}
		status, ok := responseInteger(resultObject["status"], 100, 599)
		if !ok {
			return nil, invalidBatchResponse()
		}
		body, ok := resultObject["body"]
		if !ok {
			return nil, invalidBatchResponse()
		}

		response.Results = append(response.Results, BatchResult{
			ID:     id,
			Status: status,
			Body:   body,
		})
	}

	return response, nil
}

func invalidBatchResponse() error {
	return &ResponseContractError{Resource: "batch"}
}

func responseInteger(value interface{}, minimum, maximum int64) (int, bool) {
	number, ok := value.(float64)
	if !ok || math.IsNaN(number) || math.IsInf(number, 0) || math.Trunc(number) != number {
		return 0, false
	}
	if number < float64(minimum) || number > float64(maximum) {
		return 0, false
	}
	return int(number), true
}

func maxInt() int {
	return int(^uint(0) >> 1)
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
