# slskr Go API Client

Official Go client library for the slskr HTTP API.

## Features

- ✅ Context-aware HTTP client
- ✅ Complete API coverage
- ✅ Type-safe responses
- ✅ Error handling
- ✅ No external dependencies (except gorilla/websocket)

## Installation

```bash
go get github.com/snapetech/slskr/client-go
```

## Quick Start

```go
package main

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/snapetech/slskr/client-go"
)

func main() {
	client := slskr.NewClient(
		"http://127.0.0.1:5030",
		"your-bearer-token",
	)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Get stats
	stats, err := client.GetStats(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Stats: %v\n", stats)

	// Create search
	search, err := client.CreateSearch(ctx, "artist name")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Search ID: %v\n", search["id"])

	// List transfers
	transfers, err := client.ListTransfers(ctx, "download", "active", 10, 0)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Active downloads: %d\n", len(transfers))
}
```

## API Reference

### Health & Info
- `Health(ctx)` - Server health
- `Version(ctx)` - Version information
- `GetCapabilities(ctx)` - API capabilities
- `GetConfig(ctx)` - Configuration
- `GetStats(ctx)` - Statistics

### Search
- `ListSearches(ctx, limit, offset)` - List searches
- `CreateSearch(ctx, query)` - Create search

### Messages
- `ListMessages(ctx, limit, offset)` - List messages
- `SendMessage(ctx, recipient, content)` - Send message

### Transfers
- `ListTransfers(ctx, direction, status, limit, offset)` - List transfers

## Error Handling

```go
stats, err := client.GetStats(ctx)
if err != nil {
	log.Printf("Error: %v", err)
}
```

## Context Usage

All methods use `context.Context` for cancellation and timeouts:

```go
ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
defer cancel()

stats, err := client.GetStats(ctx)
```

## Configuration

```go
client := slskr.NewClient(baseURL, token)
client.Timeout = 60 * time.Second  // Custom timeout
```

## Contributing

Contributions welcome!

## License

MIT - See LICENSE for details
