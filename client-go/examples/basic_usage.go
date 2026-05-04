package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/your-org/soulseekr"
)

func main() {
	// Initialize client
	client := soulseekr.NewClient("http://localhost:8080", "your-api-key-here")
	client.HTTPClient.Timeout = 30 * time.Second

	ctx := context.Background()

	// Check server health
	health, err := client.Health(ctx)
	if err != nil {
		log.Fatalf("Error checking health: %v", err)
	}
	fmt.Printf("Server status: %v\n", health)

	// Get API version
	version, err := client.Version(ctx)
	if err != nil {
		log.Fatalf("Error getting version: %v", err)
	}
	fmt.Printf("API version: %v\n", version)

	// Get capabilities
	caps, err := client.GetCapabilities(ctx)
	if err != nil {
		log.Fatalf("Error getting capabilities: %v", err)
	}
	if endpoints, ok := caps["endpoints"].([]interface{}); ok {
		fmt.Printf("Endpoints available: %d\n", len(endpoints))
	}

	// Get server stats
	stats, err := client.GetStats(ctx)
	if err != nil {
		log.Fatalf("Error getting stats: %v", err)
	}
	fmt.Printf("Total searches: %v\n", stats["total_searches"])
	fmt.Printf("Active transfers: %v\n", stats["active_transfers"])

	// List searches
	searches, err := client.ListSearches(ctx, 10, 0)
	if err != nil {
		log.Fatalf("Error listing searches: %v", err)
	}
	fmt.Printf("\nFound %d recent searches\n", len(searches))
	for i, s := range searches {
		if i >= 3 {
			break
		}
		fmt.Printf("  - Query: %v\n", s["query"])
		fmt.Printf("    Status: %v\n", s["status"])
	}

	// Create a new search
	search, err := client.CreateSearch(ctx, "beethoven symphony")
	if err != nil {
		log.Fatalf("Error creating search: %v", err)
	}
	fmt.Printf("\nCreated search: %v\n", search["id"])
	fmt.Printf("Query: %v\n", search["query"])

	// List messages
	messages, err := client.ListMessages(ctx, 5, 0)
	if err != nil {
		log.Fatalf("Error listing messages: %v", err)
	}
	fmt.Printf("\nMessages: %d\n", len(messages))
	for i, m := range messages {
		if i >= 2 {
			break
		}
		fmt.Printf("  - From: %v\n", m["username"])
		fmt.Printf("    Content: %v\n", m["content"])
	}

	// List transfers
	transfers, err := client.ListTransfers(ctx, "", "", 5, 0)
	if err != nil {
		log.Fatalf("Error listing transfers: %v", err)
	}
	fmt.Printf("\nTransfers: %d\n", len(transfers))
	for i, t := range transfers {
		if i >= 2 {
			break
		}
		fmt.Printf("  - File: %v\n", t["filename"])
		fmt.Printf("    Status: %v\n", t["status"])
	}

	// List users
	users, err := client.ListUsers(ctx, 5, 0)
	if err != nil {
		log.Fatalf("Error listing users: %v", err)
	}
	fmt.Printf("\nUsers: %d\n", len(users))

	// List rooms
	rooms, err := client.ListRooms(ctx)
	if err != nil {
		log.Fatalf("Error listing rooms: %v", err)
	}
	fmt.Printf("Rooms: %d\n", len(rooms))

	// List shares
	shares, err := client.ListShares(ctx, 5, 0)
	if err != nil {
		log.Fatalf("Error listing shares: %v", err)
	}
	fmt.Printf("Shares: %d\n", len(shares))
}
