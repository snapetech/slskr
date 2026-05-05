//go:build examples

package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/snapetech/slskr/client-go"
)

func main() {
	client := slskr.NewClient("http://localhost:8080", "your-api-key-here")
	client.HTTPClient.Timeout = 30 * time.Second

	ctx := context.Background()

	// Create batch builder
	batch := client.NewBatchBuilder()

	// Add multiple operations
	opID1 := "get-stats"
	opID2 := "get-caps"
	opID3 := "get-config"

	batch.Get("/api/stats", &opID1)
	batch.Get("/api/capabilities", &opID2)
	batch.Get("/api/config", &opID3)

	// Add search operations
	opID4 := "search-1"
	opID5 := "search-2"

	batch.Post("/api/searches", map[string]interface{}{
		"query": "bach concerto",
	}, &opID4)

	batch.Post("/api/searches", map[string]interface{}{
		"query": "mozart sonata",
	}, &opID5)

	fmt.Printf("Executing batch of %d operations...\n", batch.Size())

	// Execute batch
	response, err := batch.Execute(ctx)
	if err != nil {
		log.Fatalf("Error executing batch: %v", err)
	}

	fmt.Printf("\nBatch completed in %dms\n", response.TotalTimeMs)
	fmt.Printf("Successful: %d\n", len(response.GetSuccessful()))
	fmt.Printf("Failed: %d\n", len(response.GetFailed()))

	// Process results
	for _, result := range response.Results {
		fmt.Printf("\nOperation: %s\n", result.ID)
		fmt.Printf("  Status: %d\n", result.Status)
		if result.IsSuccess() {
			fmt.Printf("  Result: %v\n", result.Body)
		} else {
			fmt.Printf("  Error: %v\n", result.Body)
		}
	}

	// Check overall success
	if response.AllSuccessful() {
		fmt.Println("\n✓ All operations successful!")
	} else {
		fmt.Printf("\n✗ %d operations failed\n", len(response.GetFailed()))
	}
}
