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

	// Create WebSocket client
	ws := client.NewWebSocketClient(true)

	ctx := context.Background()

	// Connect to WebSocket
	err := ws.Connect(ctx)
	if err != nil {
		log.Fatalf("Error connecting: %v", err)
	}
	defer ws.Disconnect(ctx)

	// Create channels for events
	messageCh := make(chan interface{}, 100)
	searchUpdateCh := make(chan interface{}, 100)
	transferUpdateCh := make(chan interface{}, 100)
	connectionCh := make(chan bool, 10)
	errorCh := make(chan error, 10)

	// Register listeners
	ws.On("message", messageCh)
	ws.On("search_update", searchUpdateCh)
	ws.On("transfer_update", transferUpdateCh)
	ws.OnConnectionChange(connectionCh)
	ws.OnError(errorCh)

	// Subscribe to topics
	topics := []string{"messages", "search_updates", "transfer_updates"}
	err = ws.Subscribe(topics...)
	if err != nil {
		log.Fatalf("Error subscribing: %v", err)
	}

	fmt.Printf("Subscribed to topics: %v\n", ws.GetSubscribedTopics())
	fmt.Println("Listening for events (press Ctrl+C to stop)...\n")

	// Listen for events
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()

	eventCount := 0
	for {
		select {
		case event := <-messageCh:
			if m, ok := event.(map[string]interface{}); ok {
				fmt.Printf("Message from %v: %v\n", m["username"], m["content"])
				eventCount++
			}

		case event := <-searchUpdateCh:
			if m, ok := event.(map[string]interface{}); ok {
				fmt.Printf("Search update: %v - %v results\n", m["query"], m["result_count"])
				eventCount++
			}

		case event := <-transferUpdateCh:
			if m, ok := event.(map[string]interface{}); ok {
				fmt.Printf("Transfer %v: %v - %v%%\n", m["id"], m["status"], m["progress"])
				eventCount++
			}

		case connected := <-connectionCh:
			if connected {
				fmt.Println("✓ WebSocket connected")
			} else {
				fmt.Println("✗ WebSocket disconnected")
			}

		case err := <-errorCh:
			fmt.Printf("Error: %v\n", err)

		case <-ticker.C:
			fmt.Printf("Events received: %d\n", eventCount)
			return
		}
	}
}
