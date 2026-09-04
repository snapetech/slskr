# slskr Go API Client

Go client library for the independent slskr HTTP API.

## Features

- ✅ Context-aware HTTP client
- ✅ Core API coverage
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

### Sessions, Users & Rooms
- `GetSessions(ctx)` / `CreateSession(ctx, kind, parameters)` - Read or connect the server session
- `PingSession(ctx, id)` / `DisconnectSession(ctx, id)` - Maintain or close the session
- `GetSessionPrivileges(ctx, id)` - Read current privileges
- `ListUsers(ctx, limit, offset)` / `GetUser(ctx, username)` - Read watched users
- `ListRooms(ctx[, limit, offset])` / `GetRoom(ctx, name)` / `JoinRoom(ctx, name)` / `LeaveRoom(ctx, name)` - Manage rooms

### Search
- `ListSearches(ctx, limit, offset)` - List searches
- `CreateSearch(ctx, query)` / `CreateSearchWithOptions(ctx, query, options)` - Create global or targeted searches
- `GetSearchDetails(ctx, searchID, limit, offset)` - Get search results

### Messages
- `ListMessages(ctx, limit, offset)` - List messages
- `GetUserMessages(ctx, username, limit[, offset])` - User messages
- `SendMessage(ctx, recipient, content)` - Send message
- `AcknowledgeMessage(ctx, messageID)` - Mark a message as read

### Transfers
- `ListTransfers(ctx, direction, status, limit, offset)` - List transfers
- `CreateTransfer(ctx, direction, peerUsername, filename)` - Queue a transfer
- `GetTransfer(ctx, transferID)` - Get transfer details
- `CancelTransfer(ctx, transferID)` - Cancel a transfer

### Browse, Events & Administration
- `BrowseUser(ctx, username, folder, limit, offset)` / `RequestBrowse(ctx, username[, folder])` - Browse a user's shares
- `GetBrowseRequests(ctx, status, limit, offset)` / `RespondToBrowseRequest(ctx, username, action, folder)` - Manage browse requests
- `GetEvents(ctx, eventType, limit, offset)` - Read recorded events
- `ListShares(ctx, limit, offset)` / `RefreshShares(ctx)` - Read or rescan shares
- `GetFilters(ctx)` / `UpdateFilters(ctx, filters)` - Read or update filters
- `GetCacheStats(ctx)` / `InvalidateCache(ctx, keys)` - Inspect or clear the MediaCore cache

### WebSocket Events

Create a WebSocket client with `NewWebSocketClient`, register event and
connection listeners, and call `Connect`. Unexpected disconnects are retried
with bounded exponential backoff; subscribed topics are restored automatically.

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

AGPL-3.0-only. See the repository [LICENSE](../LICENSE) and [NOTICE](../NOTICE)
files for details.
