package slskr

import (
	"encoding/json"
	"fmt"
	"math"
	"strings"
)

// APIError reports an HTTP API response with a status code of 400 or greater.
//
// Error preserves the client's existing "API error: <status> - <body>"
// formatting, while Status, Code, and Details expose structured error data to
// callers without requiring string parsing.
type APIError struct {
	Status  int
	Code    string
	Details string

	message string
	cause   error
}

// ApiError is an alias for APIError for callers using the spelling used by
// the other SDKs.
type ApiError = APIError

func (e *APIError) Error() string {
	if e == nil {
		return "<nil>"
	}
	if e.message != "" {
		return e.message
	}
	if e.Code != "" {
		return fmt.Sprintf("API error: %d - %s", e.Status, e.Code)
	}
	return fmt.Sprintf("API error: %d", e.Status)
}

func (e *APIError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.cause
}

// IsClientError reports whether the API returned a 4xx status.
func (e *APIError) IsClientError() bool {
	return e != nil && e.Status >= 400 && e.Status < 500
}

// IsServerError reports whether the API returned a 5xx or higher status.
func (e *APIError) IsServerError() bool {
	return e != nil && e.Status >= 500
}

// IsNotFound reports whether the API returned 404 Not Found.
func (e *APIError) IsNotFound() bool {
	return e != nil && e.Status == 404
}

// IsUnauthorized reports whether the API returned 401 Unauthorized.
func (e *APIError) IsUnauthorized() bool {
	return e != nil && e.Status == 401
}

// IsForbidden reports whether the API returned 403 Forbidden.
func (e *APIError) IsForbidden() bool {
	return e != nil && e.Status == 403
}

// IsConflict reports whether the API returned 409 Conflict.
func (e *APIError) IsConflict() bool {
	return e != nil && e.Status == 409
}

func newAPIError(status int, body []byte) *APIError {
	apiError := &APIError{
		Status:  status,
		Code:    fmt.Sprintf("HTTP %d", status),
		message: fmt.Sprintf("API error: %d - %s", status, redactErrorBody(body)),
	}

	var payload map[string]interface{}
	if err := json.Unmarshal(body, &payload); err != nil {
		return apiError
	}
	for _, key := range []string{"code", "error"} {
		if code, ok := payload[key].(string); ok && strings.TrimSpace(code) != "" {
			apiError.Code = code
			break
		}
	}
	if details, ok := payload["details"].(string); ok {
		apiError.Details = details
	}
	return apiError
}

func newAPIErrorFromCause(status int, cause error) *APIError {
	return &APIError{
		Status:  status,
		Code:    fmt.Sprintf("HTTP %d", status),
		message: fmt.Sprintf("API error: %d - %s", status, cause),
		cause:   cause,
	}
}

// ResponseContractError reports a successful HTTP response that does not
// match the JSON contract expected by the client.
type ResponseContractError struct {
	Resource string
}

func (e *ResponseContractError) Error() string {
	resource := e.Resource
	if resource == "" {
		resource = "API"
	}
	return fmt.Sprintf("API returned an invalid %s response", resource)
}

func validResponseIdentifier(value interface{}) bool {
	switch typed := value.(type) {
	case string:
		return strings.TrimSpace(typed) != ""
	case float64:
		return math.IsInf(typed, 0) == false &&
			math.IsNaN(typed) == false &&
			math.Trunc(typed) == typed &&
			math.Abs(typed) <= 9007199254740991
	default:
		return false
	}
}

func requireResponseIdentifier(result map[string]interface{}, resource string, keys ...string) error {
	if _, ok := responseIdentifier(result, keys...); ok {
		return nil
	}
	return &ResponseContractError{Resource: resource}
}

func responseIdentifier(result map[string]interface{}, keys ...string) (interface{}, bool) {
	for _, key := range keys {
		if value, ok := result[key]; ok && validResponseIdentifier(value) {
			return value, true
		}
	}
	return nil, false
}

func normalizeResponseIdentifier(result map[string]interface{}, resource, canonical string, aliases ...string) error {
	keys := make([]string, 0, len(aliases)+1)
	keys = append(keys, canonical)
	keys = append(keys, aliases...)
	value, ok := responseIdentifier(result, keys...)
	if !ok {
		return &ResponseContractError{Resource: resource}
	}
	if !validResponseIdentifier(result[canonical]) {
		result[canonical] = value
	}
	return nil
}

func requireResponseText(result map[string]interface{}, resource string, keys ...string) error {
	for _, key := range keys {
		if value, ok := result[key].(string); ok && strings.TrimSpace(value) != "" {
			return nil
		}
	}
	return &ResponseContractError{Resource: resource}
}
