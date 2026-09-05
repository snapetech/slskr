package slskr

import "fmt"

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
