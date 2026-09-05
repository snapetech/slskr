package slskr

import (
	"fmt"
	"math"
	"strings"
)

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
