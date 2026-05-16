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

func searchWithContext(ctx context.Context, client *slskr.Client, query string) (map[string]interface{}, error) {
	// Create a timeout context if not already set
	if _, ok := ctx.Deadline(); !ok {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, 30*time.Second)
		defer cancel()
	}

	return client.CreateSearch(ctx, query)
}

func concurrentSearches(ctx context.Context, client *slskr.Client, queries []string) {
	var wg sync.WaitGroup
	results := make(chan map[string]interface{}, len(queries))

	for _, q := range queries {
		wg.Add(1)
		go func(query string) {
			defer wg.Done()
			result, err := searchWithContext(ctx, client, query)
			if err != nil {
				fmt.Printf("Error searching '%s': %v\n", query, err)
				return
			}
			results <- result
		}(q)
	}

	go func() {
		wg.Wait()
		close(results)
	}()

	count := 0
	for result := range results {
		count++
		fmt.Printf("Search created: %v\n", result["id"])
	}
	fmt.Printf("Completed %d/%d searches\n", count, len(queries))
}

func listAllResults(ctx context.Context, client *slskr.Client, searchID string) {
	limit := 50
	offset := 0
	total := 0

	for {
		result, err := client.ListSearches(ctx, limit, offset)
		if err != nil {
			fmt.Printf("Error fetching results: %v\n", err)
			break
		}

		if len(result) == 0 {
			break
		}

		total += len(result)
		offset += len(result)

		fmt.Printf("Fetched %d results (total: %d)...\n", len(result), total)

		if len(result) < limit {
			break
		}
	}

	fmt.Printf("Total results: %d\n", total)
}

func batchOperationsExample(ctx context.Context, client *slskr.Client) {
	batch := client.NewBatchBuilder()

	// Add various operations
	batch.Get("/api/stats", nil)
	batch.Get("/api/config", nil)
	batch.Get("/api/capabilities", nil)

	batch.Post("/api/searches", map[string]interface{}{
		"query": "radiohead ok computer",
	}, nil)

	batch.Post("/api/searches", map[string]interface{}{
		"query": "pink floyd dark side",
	}, nil)

	fmt.Printf("Executing batch of %d operations...\n", batch.Size())

	response, err := batch.Execute(ctx)
	if err != nil {
		log.Printf("Batch error: %v\n", err)
		return
	}

	fmt.Printf("Batch completed in %dms\n", response.TotalTimeMs)
	fmt.Printf("Successful: %d\n", len(response.GetSuccessful()))
	fmt.Printf("Failed: %d\n", len(response.GetFailed()))
}

func errorHandlingExample(ctx context.Context, client *slskr.Client) {
	fmt.Println("\n=== Error Handling Example ===")

	// Try invalid endpoint in batch
	batch := client.NewBatchBuilder()
	batch.Get("/api/invalid-endpoint", nil)
	batch.Get("/api/stats", nil)

	response, err := batch.Execute(ctx)
	if err != nil {
		fmt.Printf("Batch execution failed: %v\n", err)
		return
	}

	for _, result := range response.Results {
		if result.IsError() {
			fmt.Printf("Operation %s failed with status %d\n", result.ID, result.Status)
		} else {
			fmt.Printf("Operation %s succeeded (status %d)\n", result.ID, result.Status)
		}
	}
}

func main() {
	client := slskr.NewClient("http://127.0.0.1:5030", "your-api-key-here")
	client.HTTPClient.Timeout = 30 * time.Second

	ctx := context.Background()

	fmt.Println("=== Advanced Usage Examples ===\n")

	// Example 1: Concurrent searches
	fmt.Println("1. Concurrent Searches")
	queries := []string{"beethoven", "mozart", "bach", "brahms"}
	concurrentSearches(ctx, client, queries)

	// Example 2: Batch operations
	fmt.Println("\n2. Batch Operations")
	batchOperationsExample(ctx, client)

	// Example 3: Error handling
	errorHandlingExample(ctx, client)

	// Example 4: List all with pagination
	fmt.Println("\n4. Pagination Example")
	listAllResults(ctx, client, "")

	// Example 5: Room operations
	fmt.Println("\n5. Room Operations")
	rooms, err := client.ListRooms(ctx)
	if err != nil {
		fmt.Printf("Error listing rooms: %v\n", err)
	} else {
		fmt.Printf("Available rooms: %d\n", len(rooms))
	}

	// Example 6: Shares
	fmt.Println("\n6. Share Management")
	shares, err := client.ListShares(ctx, 10, 0)
	if err != nil {
		fmt.Printf("Error listing shares: %v\n", err)
	} else {
		fmt.Printf("Shared files: %d\n", len(shares))
	}

	// Example 7: Filters
	fmt.Println("\n7. Search Filters")
	filters, err := client.GetFilters(ctx)
	if err != nil {
		fmt.Printf("Error getting filters: %v\n", err)
	} else {
		fmt.Printf("Filters: %v\n", filters)
	}
}
