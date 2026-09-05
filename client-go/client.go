// Package slskr provides an HTTP client for the slskr API
package slskr

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"
)

var sensitiveErrorFieldPattern = regexp.MustCompile(`(?i)("?(api[-_]?key|authorization|credential|pass(word)?|secret|session|token)"?\s*[:=]\s*)("[^"]*"|[^,\s}\]]+)`)

const (
	maxHTTPResponseBytes = 8 * 1024 * 1024
	maxHTTPErrorBytes    = 64 * 1024
)

// Client is the main HTTP client for slskr API
type Client struct {
	BaseURL    string
	initErr    error
	Token      string
	HTTPClient *http.Client
	Timeout    time.Duration
}

// NewClient creates a new slskr client
func NewClient(baseURL, token string) *Client {
	normalizedBaseURL, err := normalizeHTTPBaseURL(baseURL)
	return &Client{
		BaseURL:    normalizedBaseURL,
		initErr:    err,
		Token:      token,
		HTTPClient: &http.Client{},
		Timeout:    30 * time.Second,
	}
}

func normalizeHTTPBaseURL(baseURL string) (string, error) {
	parsed, err := url.Parse(baseURL)
	if err != nil || parsed.Host == "" || (parsed.Scheme != "http" && parsed.Scheme != "https") || parsed.User != nil {
		return "", fmt.Errorf("base URL must be an absolute HTTP or HTTPS URL without credentials")
	}
	parsed.Path = strings.TrimRight(parsed.Path, "/")
	parsed.RawQuery = ""
	parsed.Fragment = ""
	return strings.TrimRight(parsed.String(), "/"), nil
}

// Health checks server health
func (c *Client) Health(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/health", false)
}

// Version gets version information
func (c *Client) Version(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/version", false)
}

// GetConfig gets current configuration
func (c *Client) GetConfig(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/config", true)
}

// GetStats gets server statistics
func (c *Client) GetStats(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/stats", true)
}

// GetCapabilities gets API capabilities
func (c *Client) GetCapabilities(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/capabilities", false)
}

// GetSessions gets the current server session snapshot as a list.
func (c *Client) GetSessions(ctx context.Context) ([]map[string]interface{}, error) {
	result, err := c.get(ctx, "/api/session", true)
	if err != nil {
		return nil, err
	}
	return []map[string]interface{}{result}, nil
}

// CreateSession connects the server session and returns its refreshed snapshot.
func (c *Client) CreateSession(ctx context.Context, kind string, parameters map[string]interface{}) (map[string]interface{}, error) {
	if kind != "server" {
		return nil, fmt.Errorf("unsupported session type: %s", kind)
	}
	var err error
	if _, hasUsername := parameters["username"]; hasUsername {
		_, err = c.put(ctx, "/api/server", parameters, true)
	} else if _, hasPassword := parameters["password"]; hasPassword {
		_, err = c.put(ctx, "/api/server", parameters, true)
	} else {
		_, err = c.post(ctx, "/api/session/connect", map[string]interface{}{}, true)
	}
	if err != nil {
		return nil, err
	}
	sessions, err := c.GetSessions(ctx)
	if err != nil {
		return nil, err
	}
	if len(sessions) == 0 {
		return map[string]interface{}{}, nil
	}
	return sessions[0], nil
}

// PingSession keeps the server session alive.
func (c *Client) PingSession(ctx context.Context, sessionID string) (map[string]interface{}, error) {
	_ = sessionID
	return c.post(ctx, "/api/session/ping", map[string]interface{}{}, true)
}

// DisconnectSession disconnects the server session.
func (c *Client) DisconnectSession(ctx context.Context, sessionID string) error {
	_ = sessionID
	_, err := c.post(ctx, "/api/session/disconnect", map[string]interface{}{}, true)
	return err
}

// GetSessionPrivileges requests and returns the current session privileges.
func (c *Client) GetSessionPrivileges(ctx context.Context, sessionID string) (map[string]interface{}, error) {
	_ = sessionID
	if _, err := c.post(ctx, "/api/session/privileges/check", map[string]interface{}{}, true); err != nil {
		return nil, err
	}
	snapshot, err := c.get(ctx, "/api/session", true)
	if err != nil {
		return nil, err
	}
	privileges := []interface{}{}
	if seconds, ok := snapshot["privileges_seconds"].(float64); ok && seconds > 0 {
		privileges = append(privileges, "privileged")
	}
	userID, _ := snapshot["username"].(string)
	return map[string]interface{}{
		"user_id":    userID,
		"privileges": privileges,
	}, nil
}

// ListSearches lists searches
func (c *Client) ListSearches(ctx context.Context, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getRawWithParams(ctx, "/api/searches", params, true)
	if err != nil {
		return nil, err
	}

	searches, err := responseArray(result, "searches", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, s := range searches {
		if m, ok := s.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
}

// SearchOptions describes optional targeting fields for a search.
type SearchOptions struct {
	Room   string
	Target string
}

// CreateSearch creates a new global search.
func (c *Client) CreateSearch(ctx context.Context, query string) (map[string]interface{}, error) {
	return c.createSearch(ctx, query, nil)
}

// CreateSearchWithOptions creates a search with optional room or target fields.
func (c *Client) CreateSearchWithOptions(ctx context.Context, query string, options SearchOptions) (map[string]interface{}, error) {
	return c.createSearch(ctx, query, &options)
}

func (c *Client) createSearch(ctx context.Context, query string, options *SearchOptions) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"query": query,
	}
	if options != nil {
		if options.Room != "" {
			body["room"] = options.Room
		}
		if options.Target != "" {
			body["target"] = options.Target
		}
	}
	result, err := c.post(ctx, "/api/searches", body, true)
	if err != nil {
		return nil, err
	}
	if searchID, hasID := result["id"]; hasID && validResponseIdentifier(searchID) {
		return result, nil
	}
	if searchID, hasSearchID := result["searchId"]; hasSearchID && validResponseIdentifier(searchID) {
		result["id"] = searchID
		return result, nil
	}
	return nil, &ResponseContractError{Resource: "search"}
}

// GetSearchDetails gets a search and its result page.
func (c *Client) GetSearchDetails(ctx context.Context, searchID string, limit, offset int) (map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))
	return c.getWithParams(ctx, fmt.Sprintf("/api/searches/%s", pathSegment(searchID)), params, true)
}

// ListTransfers lists transfers
func (c *Client) ListTransfers(ctx context.Context, direction, status string, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	if direction != "" {
		params.Set("direction", transferDirectionValue(direction))
	}
	if status != "" {
		params.Set("status", status)
	}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, "/api/transfers", params, true)
	if err != nil {
		return nil, err
	}

	transfers, err := responseArray(result, "transfers", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, t := range transfers {
		if m, ok := t.(map[string]interface{}); ok {
			if err := requireResponseIdentifier(m, "transfer", "id"); err != nil {
				return nil, err
			}
			out = append(out, m)
		}
	}
	return out, nil
}

// CreateTransfer queues a transfer.
func (c *Client) CreateTransfer(ctx context.Context, direction, peerUsername, filename string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"direction":     transferDirectionNumber(direction),
		"peer_username": peerUsername,
		"filename":      filename,
	}
	result, err := c.post(ctx, "/api/transfers", body, true)
	if err != nil {
		return nil, err
	}
	if err := requireResponseIdentifier(result, "transfer", "id"); err != nil {
		return nil, err
	}
	return result, nil
}

// GetTransfer gets transfer details.
func (c *Client) GetTransfer(ctx context.Context, transferID string) (map[string]interface{}, error) {
	result, err := c.get(ctx, fmt.Sprintf("/api/transfers/%s", pathSegment(transferID)), true)
	if err != nil {
		return nil, err
	}
	if err := requireResponseIdentifier(result, "transfer", "id"); err != nil {
		return nil, err
	}
	return result, nil
}

// CancelTransfer cancels a transfer.
func (c *Client) CancelTransfer(ctx context.Context, transferID string) error {
	_, err := c.delete(ctx, fmt.Sprintf("/api/transfers/%s", pathSegment(transferID)), true)
	return err
}

// ListMessages lists messages
func (c *Client) ListMessages(ctx context.Context, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, "/api/messages", params, true)
	if err != nil {
		return nil, err
	}

	messages, err := responseArray(result, "messages", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, m := range messages {
		if msg, ok := m.(map[string]interface{}); ok {
			if err := requireResponseIdentifier(msg, "message", "id"); err != nil {
				return nil, err
			}
			out = append(out, msg)
		}
	}
	return out, nil
}

// GetUserMessages gets messages from a specific user. An optional offset may
// be supplied for paginated reads.
func (c *Client) GetUserMessages(ctx context.Context, username string, limit int, offsets ...int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	offset := 0
	if len(offsets) > 0 {
		offset = offsets[0]
	}
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, fmt.Sprintf("/api/messages/%s", pathSegment(username)), params, true)
	if err != nil {
		return nil, err
	}

	messages, err := responseArray(result, "messages", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, m := range messages {
		if msg, ok := m.(map[string]interface{}); ok {
			if err := requireResponseIdentifier(msg, "message", "id"); err != nil {
				return nil, err
			}
			out = append(out, msg)
		}
	}
	return out, nil
}

// SendMessage sends a message to user
func (c *Client) SendMessage(ctx context.Context, recipient, content string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"username": recipient,
		"body":     content,
	}
	result, err := c.post(ctx, "/api/messages", body, true)
	if err != nil {
		return nil, err
	}
	if err := requireResponseIdentifier(result, "message", "id"); err != nil {
		return nil, err
	}
	return result, nil
}

// AcknowledgeMessage marks message as acknowledged
func (c *Client) AcknowledgeMessage(ctx context.Context, messageID string) error {
	_, err := c.post(ctx, fmt.Sprintf("/api/messages/%s/ack", pathSegment(messageID)), nil, true)
	return err
}

// ============================================================================
// Users
// ============================================================================

// GetUser gets user info
func (c *Client) GetUser(ctx context.Context, username string) (map[string]interface{}, error) {
	return c.get(ctx, fmt.Sprintf("/api/users/%s/info", pathSegment(username)), true)
}

// ListUsers lists users
func (c *Client) ListUsers(ctx context.Context, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, "/api/users", params, true)
	if err != nil {
		return nil, err
	}

	users, err := responseArray(result, "users", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, u := range users {
		if m, ok := u.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
}

// ============================================================================
// Rooms
// ============================================================================

// ListRooms lists chat rooms. Optional limit and offset values enable
// pagination while preserving the unbounded legacy call shape.
func (c *Client) ListRooms(ctx context.Context, pagination ...int) ([]map[string]interface{}, error) {
	params := url.Values{}
	if len(pagination) > 0 {
		params.Set("limit", fmt.Sprintf("%d", pagination[0]))
	}
	if len(pagination) > 1 {
		params.Set("offset", fmt.Sprintf("%d", pagination[1]))
	}
	result, err := c.getWithParams(ctx, "/api/rooms", params, true)
	if err != nil {
		return nil, err
	}

	rooms, err := responseArray(result, "rooms", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, r := range rooms {
		if m, ok := r.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
}

// GetRoom gets room info
func (c *Client) GetRoom(ctx context.Context, roomID string) (map[string]interface{}, error) {
	return c.get(ctx, fmt.Sprintf("/api/rooms/%s", pathSegment(roomID)), true)
}

// JoinRoom joins a room
func (c *Client) JoinRoom(ctx context.Context, roomName string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"name": roomName,
	}
	return c.post(ctx, fmt.Sprintf("/api/rooms/%s/join", pathSegment(roomName)), body, true)
}

// LeaveRoom leaves a room
func (c *Client) LeaveRoom(ctx context.Context, roomID string) error {
	_, err := c.delete(ctx, fmt.Sprintf("/api/rooms/%s/join", pathSegment(roomID)), true)
	return err
}

// ============================================================================
// Shares
// ============================================================================

// ListShares lists shared files
func (c *Client) ListShares(ctx context.Context, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, "/api/shares", params, true)
	if err != nil {
		return nil, err
	}

	shares, err := responseArray(result, "shares", "local", "entries")
	if err != nil {
		return nil, err
	}

	var out []map[string]interface{}
	for _, s := range shares {
		if m, ok := s.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
}

// RefreshShares refreshes the share list
func (c *Client) RefreshShares(ctx context.Context) (map[string]interface{}, error) {
	return c.post(ctx, "/api/shares/rescan", nil, true)
}

// ============================================================================
// Filters
// ============================================================================

// GetFilters gets search filters
func (c *Client) GetFilters(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/config/download-filter", true)
}

// UpdateFilters updates search filters
func (c *Client) UpdateFilters(ctx context.Context, filters map[string]interface{}) (map[string]interface{}, error) {
	return c.put(ctx, "/api/config/download-filter", filters, true)
}

// ============================================================================
// Browse, events, and cache
// ============================================================================

// BrowseUser gets a user's shared files, optionally filtered to a folder.
func (c *Client) BrowseUser(ctx context.Context, username, folder string, limit, offset int) (map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))
	if folder != "" {
		params.Set("folder", folder)
	}
	return c.getWithParams(ctx, fmt.Sprintf("/api/users/%s/browse", pathSegment(username)), params, true)
}

// RequestBrowse requests a fresh browse listing from a user. Supplying an
// optional folder requests that specific folder instead of the user's root.
func (c *Client) RequestBrowse(ctx context.Context, username string, folders ...string) (map[string]interface{}, error) {
	path := fmt.Sprintf("/api/users/%s/browse/request", pathSegment(username))
	body := map[string]interface{}{}
	if len(folders) > 0 {
		path = fmt.Sprintf("/api/users/%s/browse/folder", pathSegment(username))
		body["folder"] = folders[0]
	}
	return c.post(ctx, path, body, true)
}

// GetBrowseRequests lists pending and completed browse requests.
func (c *Client) GetBrowseRequests(ctx context.Context, status string, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))
	if status != "" {
		params.Set("status", status)
	}
	result, err := c.getWithParams(ctx, "/api/browse/requests", params, true)
	if err != nil {
		return nil, err
	}
	requests, err := responseArray(result, "requests", "entries")
	if err != nil {
		return nil, err
	}
	var out []map[string]interface{}
	for _, request := range requests {
		if object, ok := request.(map[string]interface{}); ok {
			out = append(out, object)
		}
	}
	return out, nil
}

// RespondToBrowseRequest accepts or rejects a browse request.
func (c *Client) RespondToBrowseRequest(ctx context.Context, username, action, folder string) (map[string]interface{}, error) {
	if action != "accept" && action != "reject" {
		return nil, fmt.Errorf("action must be %q or %q", "accept", "reject")
	}
	path := fmt.Sprintf("/api/users/%s/browse/folder", pathSegment(username))
	body := map[string]interface{}{"folder": folder}
	if action == "reject" {
		path = fmt.Sprintf("/api/users/%s/browse/cancel", pathSegment(username))
		body = map[string]interface{}{"reason": "rejected by client"}
	}
	return c.post(ctx, path, body, true)
}

// GetEvents lists recorded events.
func (c *Client) GetEvents(ctx context.Context, eventType string, limit, offset int) ([]map[string]interface{}, error) {
	return c.GetEventsWithFilters(ctx, eventType, "", "", limit, offset)
}

// GetEventsWithFilters lists recorded events filtered by kind, topic, and text.
// The existing GetEvents method remains the shorthand for kind-only filtering.
func (c *Client) GetEventsWithFilters(ctx context.Context, eventType, topic, query string, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))
	if eventType != "" {
		params.Set("kind", eventType)
	}
	if topic != "" {
		params.Set("topic", topic)
	}
	if query != "" {
		params.Set("q", query)
	}
	result, err := c.getRawWithParams(ctx, "/api/events", params, true)
	if err != nil {
		return nil, err
	}
	events, err := responseArray(result, "events", "entries")
	if err != nil {
		return nil, err
	}
	var out []map[string]interface{}
	for _, event := range events {
		if object, ok := event.(map[string]interface{}); ok {
			if err := requireResponseIdentifier(object, "event", "id"); err != nil {
				return nil, err
			}
			if err := requireResponseText(object, "event", "type", "kind"); err != nil {
				return nil, err
			}
			out = append(out, object)
		}
	}
	return out, nil
}

// GetCacheStats gets MediaCore retrieval cache statistics.
func (c *Client) GetCacheStats(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/mediacore/retrieve/stats", true)
}

// InvalidateCache clears selected MediaCore cache keys, or the complete cache.
func (c *Client) InvalidateCache(ctx context.Context, keys []string) (map[string]interface{}, error) {
	if keys == nil {
		keys = []string{}
	}
	return c.post(ctx, "/api/mediacore/retrieve/cache/clear", map[string]interface{}{"keys": keys}, true)
}

// ============================================================================
// Internal Methods
// ============================================================================

func (c *Client) get(ctx context.Context, path string, auth bool) (map[string]interface{}, error) {
	return c.getWithParams(ctx, path, nil, auth)
}

func (c *Client) getWithParams(ctx context.Context, path string, params url.Values, auth bool) (map[string]interface{}, error) {
	result, err := c.getRawWithParams(ctx, path, params, auth)
	if err != nil {
		return nil, err
	}

	object, ok := result.(map[string]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format: expected JSON object")
	}
	return object, nil
}

func (c *Client) getRawWithParams(ctx context.Context, path string, params url.Values, auth bool) (interface{}, error) {
	if c.initErr != nil {
		return nil, c.initErr
	}
	fullURL := c.BaseURL + path
	if params != nil && len(params) > 0 {
		fullURL += "?" + params.Encode()
	}

	req, err := http.NewRequestWithContext(ctx, "GET", fullURL, nil)
	if err != nil {
		return nil, err
	}

	return c.doJSON(req, auth)
}

func (c *Client) post(ctx context.Context, path string, body interface{}, auth bool) (map[string]interface{}, error) {
	if c.initErr != nil {
		return nil, c.initErr
	}
	fullURL := c.BaseURL + path

	bodyBytes, err := json.Marshal(body)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", fullURL, bytes.NewReader(bodyBytes))
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", "application/json")
	return c.do(req, auth)
}

func (c *Client) put(ctx context.Context, path string, body interface{}, auth bool) (map[string]interface{}, error) {
	if c.initErr != nil {
		return nil, c.initErr
	}
	fullURL := c.BaseURL + path

	bodyBytes, err := json.Marshal(body)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPut, fullURL, bytes.NewReader(bodyBytes))
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", "application/json")
	return c.do(req, auth)
}

func (c *Client) delete(ctx context.Context, path string, auth bool) (map[string]interface{}, error) {
	if c.initErr != nil {
		return nil, c.initErr
	}
	fullURL := c.BaseURL + path
	req, err := http.NewRequestWithContext(ctx, http.MethodDelete, fullURL, nil)
	if err != nil {
		return nil, err
	}

	return c.do(req, auth)
}

func (c *Client) do(req *http.Request, auth bool) (map[string]interface{}, error) {
	result, err := c.doJSON(req, auth)
	if err != nil {
		return nil, err
	}

	object, ok := result.(map[string]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format: expected JSON object")
	}
	return object, nil
}

func (c *Client) doJSON(req *http.Request, auth bool) (interface{}, error) {
	if c.Timeout > 0 {
		ctx, cancel := context.WithTimeout(req.Context(), c.Timeout)
		defer cancel()
		req = req.WithContext(ctx)
	}
	if auth {
		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", c.Token))
	}

	httpClient := http.Client{}
	if c.HTTPClient != nil {
		httpClient = *c.HTTPClient
	}
	httpClient.CheckRedirect = func(_ *http.Request, _ []*http.Request) error {
		return fmt.Errorf("refusing HTTP redirects")
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		bodyBytes, err := readBoundedBody(resp, maxHTTPErrorBytes)
		if err != nil {
			return nil, fmt.Errorf("API error: %d - %w", resp.StatusCode, err)
		}
		return nil, fmt.Errorf("API error: %d - %s", resp.StatusCode, redactErrorBody(bodyBytes))
	}

	bodyBytes, err := readBoundedBody(resp, maxHTTPResponseBytes)
	if err != nil {
		return nil, err
	}
	if len(bodyBytes) == 0 {
		return map[string]interface{}{}, nil
	}
	return decodeJSONResponse(bodyBytes)
}

func decodeJSONResponse(body []byte) (interface{}, error) {
	trimmed := bytes.TrimSpace(body)
	if len(trimmed) == 0 {
		return map[string]interface{}{}, nil
	}

	return decodeJSONValue(trimmed)
}

func decodeJSONValue(raw []byte) (interface{}, error) {
	raw = bytes.TrimSpace(raw)
	if len(raw) == 0 {
		return nil, fmt.Errorf("invalid JSON response")
	}

	switch raw[0] {
	case '{':
		var fields map[string]json.RawMessage
		if err := json.Unmarshal(raw, &fields); err != nil {
			return nil, err
		}
		result := make(map[string]interface{}, len(fields))
		for key, value := range fields {
			decoded, err := decodeJSONValue(value)
			if err != nil {
				return nil, err
			}
			result[key] = decoded
		}
		return result, nil
	case '[':
		var values []json.RawMessage
		if err := json.Unmarshal(raw, &values); err != nil {
			return nil, err
		}
		result := make([]interface{}, 0, len(values))
		for _, value := range values {
			decoded, err := decodeJSONValue(value)
			if err != nil {
				return nil, err
			}
			result = append(result, decoded)
		}
		return result, nil
	case '"':
		var result string
		if err := json.Unmarshal(raw, &result); err != nil {
			return nil, err
		}
		return result, nil
	case 't', 'f':
		var result bool
		if err := json.Unmarshal(raw, &result); err != nil {
			return nil, err
		}
		return result, nil
	case 'n':
		if !bytes.Equal(raw, []byte("null")) {
			return nil, fmt.Errorf("invalid JSON response")
		}
		return nil, nil
	default:
		var result float64
		if err := json.Unmarshal(raw, &result); err != nil {
			return nil, err
		}
		return result, nil
	}
}

func responseArray(value interface{}, keys ...string) ([]interface{}, error) {
	var values []interface{}
	if values, ok := value.([]interface{}); ok {
		return validateResponseArray(values)
	}
	object, ok := value.(map[string]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format: expected JSON array or object")
	}
	for _, key := range keys {
		if candidate, ok := object[key].([]interface{}); ok {
			values = candidate
			return validateResponseArray(values)
		}
	}
	return nil, fmt.Errorf("unexpected response format: missing array field")
}

func validateResponseArray(values []interface{}) ([]interface{}, error) {
	for _, value := range values {
		if _, ok := value.(map[string]interface{}); !ok {
			return nil, fmt.Errorf("unexpected response format: array entries must be JSON objects")
		}
	}
	return values, nil
}

func transferDirectionValue(direction string) string {
	switch strings.ToLower(strings.TrimSpace(direction)) {
	case "download":
		return "0"
	case "upload":
		return "1"
	default:
		return direction
	}
}

func transferDirectionNumber(direction string) int {
	switch strings.ToLower(strings.TrimSpace(direction)) {
	case "upload":
		return 1
	default:
		return 0
	}
}

func readBoundedBody(resp *http.Response, maximum int64) ([]byte, error) {
	if resp.ContentLength > maximum {
		return nil, fmt.Errorf("HTTP response body exceeds %d bytes", maximum)
	}
	body, err := io.ReadAll(io.LimitReader(resp.Body, maximum+1))
	if err != nil {
		return nil, err
	}
	if int64(len(body)) > maximum {
		return nil, fmt.Errorf("HTTP response body exceeds %d bytes", maximum)
	}
	return body, nil
}

func pathSegment(value string) string {
	return url.PathEscape(value)
}

func redactErrorBody(body []byte) string {
	var decoded map[string]interface{}
	if err := json.Unmarshal(body, &decoded); err == nil {
		redactJSONValue(decoded)
		redacted, err := json.Marshal(decoded)
		if err == nil {
			return string(redacted)
		}
	}

	var decodedList []map[string]interface{}
	if err := json.Unmarshal(body, &decodedList); err == nil {
		for _, item := range decodedList {
			redactJSONValue(item)
		}
		redacted, err := json.Marshal(decodedList)
		if err == nil {
			return string(redacted)
		}
	}

	return sensitiveErrorFieldPattern.ReplaceAllString(string(body), `${1}[REDACTED]`)
}

func redactJSONValue(value interface{}) {
	switch typed := value.(type) {
	case map[string]interface{}:
		for key, nested := range typed {
			if isSensitiveField(key) {
				typed[key] = "[REDACTED]"
			} else {
				redactJSONValue(nested)
			}
		}
	case []interface{}:
		for _, nested := range typed {
			redactJSONValue(nested)
		}
	}
}

func isSensitiveField(field string) bool {
	normalized := strings.ToLower(strings.ReplaceAll(field, "_", "-"))
	return strings.Contains(normalized, "token") ||
		strings.Contains(normalized, "secret") ||
		strings.Contains(normalized, "password") ||
		strings.Contains(normalized, "pass") ||
		strings.Contains(normalized, "api-key") ||
		strings.Contains(normalized, "authorization") ||
		strings.Contains(normalized, "credential") ||
		strings.Contains(normalized, "session")
}
