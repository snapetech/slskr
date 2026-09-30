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
	client := slskr.NewClient("http://127.0.0.1:5030", "your-api-key-here")

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
	searchResultCh := make(chan interface{}, 100)
	transferStartedCh := make(chan interface{}, 100)
	connectionCh := make(chan bool, 10)
	errorCh := make(chan error, 10)

	// Register listeners
	ws.On("message.received", messageCh)
	ws.On("search.result", searchResultCh)
	ws.On("transfer.started", transferStartedCh)
	ws.OnConnectionChange(connectionCh)
	ws.OnError(errorCh)

	// Subscribe to topics
	topics := []string{"message.received", "search.result", "transfer.started"}
	err = ws.Subscribe(topics...)
	if err != nil {
		log.Fatalf("Error subscribing: %v", err)
	}

	fmt.Printf("Subscribed to topics: %v\n", ws.GetSubscribedTopics())
	fmt.Println("Listening for events (press Ctrl+C to stop)...")

	// Listen for events
	ticker := time.NewTicker(10 * time.Second)
	defer ticker.Stop()

	eventCount := 0
	for {
		select {
		case event := <-messageCh:
			if m, ok := event.(map[string]interface{}); ok {
				data, _ := m["data"].(map[string]interface{})
				fmt.Printf("Message from %v: %v\n", data["username"], data["body"])
				eventCount++
			}

		case event := <-searchResultCh:
			if m, ok := event.(map[string]interface{}); ok {
				fmt.Printf("Search result event: %v\n", m["data"])
				eventCount++
			}

		case event := <-transferStartedCh:
			if m, ok := event.(map[string]interface{}); ok {
				data, _ := m["data"].(map[string]interface{})
				fmt.Printf("Transfer %v: %v - %v%%\n", data["id"], data["status"], data["progress_percent"])
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
