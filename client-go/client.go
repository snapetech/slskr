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
	"time"
)

// Client is the main HTTP client for slskr API
type Client struct {
	BaseURL    string
	Token      string
	HTTPClient *http.Client
	Timeout    time.Duration
}

// NewClient creates a new slskr client
func NewClient(baseURL, token string) *Client {
	return &Client{
		BaseURL: baseURL,
		Token:   token,
		HTTPClient: &http.Client{
			Timeout: 30 * time.Second,
		},
		Timeout: 30 * time.Second,
	}
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

// ListSearches lists searches
func (c *Client) ListSearches(ctx context.Context, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))
	params.Set("offset", fmt.Sprintf("%d", offset))

	result, err := c.getWithParams(ctx, "/api/searches", params, true)
	if err != nil {
		return nil, err
	}

	searches, ok := result["searches"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
	}

	var out []map[string]interface{}
	for _, s := range searches {
		if m, ok := s.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
}

// CreateSearch creates a new search
func (c *Client) CreateSearch(ctx context.Context, query string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"query": query,
	}
	return c.post(ctx, "/api/searches", body, true)
}

// ListTransfers lists transfers
func (c *Client) ListTransfers(ctx context.Context, direction, status string, limit, offset int) ([]map[string]interface{}, error) {
	params := url.Values{}
	if direction != "" {
		params.Set("direction", direction)
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

	transfers, ok := result["transfers"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
	}

	var out []map[string]interface{}
	for _, t := range transfers {
		if m, ok := t.(map[string]interface{}); ok {
			out = append(out, m)
		}
	}
	return out, nil
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

	messages, ok := result["messages"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
	}

	var out []map[string]interface{}
	for _, m := range messages {
		if msg, ok := m.(map[string]interface{}); ok {
			out = append(out, msg)
		}
	}
	return out, nil
}

// GetUserMessages gets messages from specific user
func (c *Client) GetUserMessages(ctx context.Context, username string, limit int) ([]map[string]interface{}, error) {
	params := url.Values{}
	params.Set("limit", fmt.Sprintf("%d", limit))

	result, err := c.getWithParams(ctx, fmt.Sprintf("/api/messages/%s", username), params, true)
	if err != nil {
		return nil, err
	}

	messages, ok := result["messages"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
	}

	var out []map[string]interface{}
	for _, m := range messages {
		if msg, ok := m.(map[string]interface{}); ok {
			out = append(out, msg)
		}
	}
	return out, nil
}

// SendMessage sends a message to user
func (c *Client) SendMessage(ctx context.Context, recipient, content string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"recipient": recipient,
		"content":   content,
	}
	return c.post(ctx, "/api/messages", body, true)
}

// AcknowledgeMessage marks message as acknowledged
func (c *Client) AcknowledgeMessage(ctx context.Context, messageID string) error {
	_, err := c.post(ctx, fmt.Sprintf("/api/messages/%s/acknowledge", messageID), nil, true)
	return err
}

// ============================================================================
// Users
// ============================================================================

// GetUser gets user info
func (c *Client) GetUser(ctx context.Context, username string) (map[string]interface{}, error) {
	return c.get(ctx, fmt.Sprintf("/api/users/%s", username), false)
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

	users, ok := result["users"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
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

// ListRooms lists chat rooms
func (c *Client) ListRooms(ctx context.Context) ([]map[string]interface{}, error) {
	result, err := c.get(ctx, "/api/rooms", true)
	if err != nil {
		return nil, err
	}

	rooms, ok := result["rooms"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
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
	return c.get(ctx, fmt.Sprintf("/api/rooms/%s", roomID), true)
}

// JoinRoom joins a room
func (c *Client) JoinRoom(ctx context.Context, roomName string) (map[string]interface{}, error) {
	body := map[string]interface{}{
		"name": roomName,
	}
	return c.post(ctx, "/api/rooms/join", body, true)
}

// LeaveRoom leaves a room
func (c *Client) LeaveRoom(ctx context.Context, roomID string) error {
	_, err := c.post(ctx, fmt.Sprintf("/api/rooms/%s/leave", roomID), nil, true)
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

	shares, ok := result["shares"].([]interface{})
	if !ok {
		return nil, fmt.Errorf("unexpected response format")
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
	return c.post(ctx, "/api/shares/refresh", nil, true)
}

// ============================================================================
// Filters
// ============================================================================

// GetFilters gets search filters
func (c *Client) GetFilters(ctx context.Context) (map[string]interface{}, error) {
	return c.get(ctx, "/api/filters", true)
}

// UpdateFilters updates search filters
func (c *Client) UpdateFilters(ctx context.Context, filters map[string]interface{}) (map[string]interface{}, error) {
	return c.post(ctx, "/api/filters", filters, true)
}

// ============================================================================
// Internal Methods
// ============================================================================

func (c *Client) get(ctx context.Context, path string, auth bool) (map[string]interface{}, error) {
	return c.getWithParams(ctx, path, nil, auth)
}

func (c *Client) getWithParams(ctx context.Context, path string, params url.Values, auth bool) (map[string]interface{}, error) {
	fullURL := c.BaseURL + path
	if params != nil && len(params) > 0 {
		fullURL += "?" + params.Encode()
	}

	req, err := http.NewRequestWithContext(ctx, "GET", fullURL, nil)
	if err != nil {
		return nil, err
	}

	return c.do(req, auth)
}

func (c *Client) post(ctx context.Context, path string, body interface{}, auth bool) (map[string]interface{}, error) {
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

func (c *Client) do(req *http.Request, auth bool) (map[string]interface{}, error) {
	if auth {
		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", c.Token))
	}

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("API error: %d - %s", resp.StatusCode, string(bodyBytes))
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}

	return result, nil
}
