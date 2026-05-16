//go:build examples

package main

import (
	"context"
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/snapetech/slskr/client-go"
)

// SearchCoordinator coordinates multiple search operations
type SearchCoordinator struct {
	client     *slskr.Client
	searches   map[string]string // searchID -> query
	resultsCnt map[string]int
	mu         sync.RWMutex
}

// NewSearchCoordinator creates a new search coordinator
func NewSearchCoordinator(client *slskr.Client) *SearchCoordinator {
	return &SearchCoordinator{
		client:     client,
		searches:   make(map[string]string),
		resultsCnt: make(map[string]int),
	}
}

// CreateSearches creates multiple searches concurrently
func (sc *SearchCoordinator) CreateSearches(ctx context.Context, queries []string) []string {
	fmt.Printf("\n=== Creating %d searches ===\n", len(queries))

	var wg sync.WaitGroup
	results := make(chan string, len(queries))
	errors := make(chan error, len(queries))

	for _, q := range queries {
		wg.Add(1)
		go func(query string) {
			defer wg.Done()

			search, err := sc.client.CreateSearch(ctx, query)
			if err != nil {
				errors <- err
				return
			}

			searchID, _ := search["id"].(string)
			sc.mu.Lock()
			sc.searches[searchID] = query
			sc.mu.Unlock()

			results <- searchID
			fmt.Printf("✓ Created search: %s (ID: %s)\n", query, searchID)
		}(q)
	}

	go func() {
		wg.Wait()
		close(results)
		close(errors)
	}()

	var searchIDs []string
	for searchID := range results {
		searchIDs = append(searchIDs, searchID)
	}

	for err := range errors {
		fmt.Printf("✗ Error creating search: %v\n", err)
	}

	return searchIDs
}

// MonitorSearches monitors search progress
func (sc *SearchCoordinator) MonitorSearches(ctx context.Context, searchIDs []string, duration time.Duration) {
	fmt.Printf("\n=== Monitoring %d searches for %v ===\n", len(searchIDs), duration)

	deadline := time.Now().Add(duration)
	ticker := time.NewTicker(500 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			if time.Now().After(deadline) {
				return
			}

			for _, sid := range searchIDs {
				query := sc.searches[sid]
				count := sc.resultsCnt[sid]
				fmt.Printf("  %s: %d results\n", query, count)
			}

		case <-ctx.Done():
			return
		}
	}
}

// MessageHandler handles message operations
type MessageHandler struct {
	client       *slskr.Client
	messagesSent int
	messagesRecv int
	mu           sync.Mutex
}

// NewMessageHandler creates a new message handler
func NewMessageHandler(client *slskr.Client) *MessageHandler {
	return &MessageHandler{
		client: client,
	}
}

// ListRecentMessages lists recent messages
func (mh *MessageHandler) ListRecentMessages(ctx context.Context, limit int) {
	messages, err := mh.client.ListMessages(ctx, limit, 0)
	if err != nil {
		fmt.Printf("Error listing messages: %v\n", err)
		return
	}

	mh.mu.Lock()
	mh.messagesRecv = len(messages)
	mh.mu.Unlock()

	fmt.Printf("\nRecent messages: %d\n", len(messages))
}

// SendBulkMessages sends multiple messages
func (mh *MessageHandler) SendBulkMessages(ctx context.Context, messages []map[string]string) {
	fmt.Printf("\n=== Sending %d messages ===\n", len(messages))

	var wg sync.WaitGroup
	results := make(chan bool, len(messages))

	for _, msg := range messages {
		wg.Add(1)
		go func(recipient, content string) {
			defer wg.Done()

			_, err := mh.client.SendMessage(ctx, recipient, content)
			if err != nil {
				fmt.Printf("✗ Failed to send to %s: %v\n", recipient, err)
				results <- false
				return
			}

			fmt.Printf("✓ Message sent to %s\n", recipient)
			results <- true
		}(msg["recipient"], msg["content"])
	}

	go func() {
		wg.Wait()
		close(results)
	}()

	sent := 0
	for success := range results {
		if success {
			sent++
		}
	}

	mh.mu.Lock()
	mh.messagesSent = sent
	mh.mu.Unlock()
}

// batchAnalytics gathers analytics using batch operations
func batchAnalytics(ctx context.Context, client *slskr.Client) map[string]interface{} {
	fmt.Println("\n=== Batch Analytics ===")

	batch := client.NewBatchBuilder()
	opStats := "stats"
	opConfig := "config"
	opCaps := "caps"

	batch.Get("/api/stats", &opStats)
	batch.Get("/api/config", &opConfig)
	batch.Get("/api/capabilities", &opCaps)

	response, err := batch.Execute(ctx)
	if err != nil {
		fmt.Printf("Error executing batch: %v\n", err)
		return nil
	}

	analytics := make(map[string]interface{})
	for _, result := range response.Results {
		if result.IsSuccess() {
			analytics[result.ID] = result.Body
			fmt.Printf("✓ %s: collected\n", result.ID)
		} else {
			fmt.Printf("✗ %s: failed with status %d\n", result.ID, result.Status)
		}
	}

	return analytics
}

// transferOperations handles transfer operations
func transferOperations(ctx context.Context, client *slskr.Client) int {
	fmt.Println("\n=== Transfer Operations ===")

	transfers, err := client.ListTransfers(ctx, "download", "active", 10, 0)
	if err != nil {
		fmt.Printf("Error listing transfers: %v\n", err)
		return 0
	}

	fmt.Printf("Active downloads: %d\n", len(transfers))

	count := 0
	for i, t := range transfers {
		if i >= 3 {
			break
		}
		if filename, ok := t["filename"].(string); ok {
			if progress, ok := t["progress"].(float64); ok {
				fmt.Printf("  - %s: %.0f%%\n", filename, progress)
			}
		}
		count++
	}

	return len(transfers)
}

// roomOperations handles room operations
func roomOperations(ctx context.Context, client *slskr.Client) int {
	fmt.Println("\n=== Room Operations ===")

	rooms, err := client.ListRooms(ctx)
	if err != nil {
		fmt.Printf("Error listing rooms: %v\n", err)
		return 0
	}

	fmt.Printf("Available rooms: %d\n", len(rooms))

	for i, r := range rooms {
		if i >= 3 {
			break
		}
		if name, ok := r["name"].(string); ok {
			if users, ok := r["user_count"].(float64); ok {
				fmt.Printf("  - %s: %d users\n", name, int(users))
			}
		}
	}

	return len(rooms)
}

// shareOperations handles share operations
func shareOperations(ctx context.Context, client *slskr.Client) int {
	fmt.Println("\n=== Share Operations ===")

	shares, err := client.ListShares(ctx, 10, 0)
	if err != nil {
		fmt.Printf("Error listing shares: %v\n", err)
		return 0
	}

	fmt.Printf("Shared files: %d\n", len(shares))

	return len(shares)
}

// websocketMonitoring monitors events via WebSocket
func websocketMonitoring(ctx context.Context, client *slskr.Client, duration time.Duration) {
	fmt.Printf("\n=== WebSocket Monitoring for %v ===\n", duration)

	ws := client.NewWebSocketClient(true)
	err := ws.Connect(ctx)
	if err != nil {
		fmt.Printf("Error connecting WebSocket: %v\n", err)
		return
	}
	defer ws.Disconnect(ctx)

	ws.Subscribe("messages", "search_updates", "transfer_updates")
	fmt.Printf("Subscribed to topics: %v\n", ws.GetSubscribedTopics())

	time.Sleep(duration)
}

func main() {
	client := slskr.NewClient("http://127.0.0.1:5030", "your-api-key-here")
	ctx := context.Background()
	ctx, cancel := context.WithTimeout(ctx, 60*time.Second)
	defer cancel()

	fmt.Println("=== slskr Integration Example ===")

	// 1. Server verification
	fmt.Println("\n=== Server Verification ===")
	health, err := client.Health(ctx)
	if err != nil {
		log.Fatalf("Error checking health: %v", err)
	}
	fmt.Printf("✓ Server status: %v\n", health["status"])

	// 2. Search coordination
	coordinator := NewSearchCoordinator(client)
	queries := []string{"radiohead", "pink floyd", "led zeppelin"}
	searchIDs := coordinator.CreateSearches(ctx, queries)

	if len(searchIDs) > 0 {
		// Monitor searches briefly
		monitorCtx, monitorCancel := context.WithTimeout(ctx, 2*time.Second)
		coordinator.MonitorSearches(monitorCtx, searchIDs, 2*time.Second)
		monitorCancel()
	}

	// 3. Message operations
	handler := NewMessageHandler(client)
	handler.ListRecentMessages(ctx, 5)

	demoMessages := []map[string]string{
		{"recipient": "demo_user_1", "content": "Test message 1"},
		{"recipient": "demo_user_2", "content": "Test message 2"},
	}
	handler.SendBulkMessages(ctx, demoMessages)

	// 4. Batch analytics
	analytics := batchAnalytics(ctx, client)
	fmt.Printf("Collected %d analytics\n", len(analytics))

	// 5. Transfer operations
	activeTransfers := transferOperations(ctx, client)

	// 6. Room operations
	roomCount := roomOperations(ctx, client)

	// 7. Share operations
	shareCount := shareOperations(ctx, client)

	// 8. WebSocket monitoring
	websocketMonitoring(ctx, client, 1*time.Second)

	// 9. Summary
	fmt.Println("\n=== Integration Summary ===")
	fmt.Printf("Searches created: %d\n", len(searchIDs))
	fmt.Printf("Messages sent: %d\n", handler.messagesSent)
	fmt.Printf("Analytics collected: %d\n", len(analytics))
	fmt.Printf("Active transfers: %d\n", activeTransfers)
	fmt.Printf("Available rooms: %d\n", roomCount)
	fmt.Printf("Shared files: %d\n", shareCount)
	fmt.Println("✓ Integration test complete")
}
