# slskr Client Libraries

Client libraries for the slskr HTTP API, providing language-specific implementations with batch operations, WebSocket support, and comprehensive examples.

## Overview

- **TypeScript/JavaScript**: Production-ready, full feature support
- **Python**: Async/await support with aiohttp, batch operations, WebSocket events
- **Go**: Concurrent operations, type-safe interfaces, batch builder pattern
- **Rust**: Native integration with HTTP API (embedded)
- **slskd automation clients**: Compatibility routes for existing `slskd_api` Python automations

## Python Client

### Installation

```bash
# From source
cd client-python
pip install -e .

# Or install dependencies
pip install aiohttp
```

### Quick Start

```python
import asyncio
from slskr import SlskrClient

async def main():
    client = SlskrClient(
        base_url="http://127.0.0.1:5030",
        token="your-api-key-here"
    )
    
    # Check server health
    health = await client.health()
    print(f"Server status: {health['status']}")
    
    # Create search
    search = await client.create_search("beethoven symphony")
    print(f"Search ID: {search['id']}")
    
    # Get search results
    details = await client.get_search_details(search['id'])
    print(f"Found {len(details['results'])} results")
    
    await client.close()

asyncio.run(main())
```

### Features

#### Core API Methods

```python
# Health & Version
await client.health()
await client.version()

# Configuration
await client.get_config()
await client.get_stats()

# Search
await client.list_searches(limit=50, offset=0)
search = await client.create_search(query="artist name")
await client.get_search_details(search_id, limit=50)

# Messages
await client.list_messages(limit=50)
await client.get_user_messages(username, limit=50)
await client.send_message(recipient, content)
await client.acknowledge_message(message_id)

# Transfers
await client.list_transfers(direction="download", status="active")
await client.create_transfer(direction, peer_username, filename)
await client.get_transfer(transfer_id)
await client.cancel_transfer(transfer_id)
```

#### Batch Operations

```python
# Build and execute batch operations
batch = client.batch.builder()
batch.get("/api/stats", op_id="stats")
batch.post("/api/searches", {"query": "query"}, op_id="search")
batch.put("/api/filters", {"enabled": True}, op_id="filters")
batch.delete("/api/transfers/123", op_id="cancel")

response = await batch.execute()

# Check results
print(f"Successful: {len(response.get_successful())}")
print(f"Failed: {len(response.get_failed())}")
print(f"Total time: {response.total_time_ms}ms")
```

#### WebSocket Events

```python
# Connect and subscribe to events
ws = await client.connect_ws()

# Define event handlers
async def handle_message(event):
    print(f"Message: {event}")

async def handle_search_update(event):
    print(f"Search update: {event}")

# Register listeners
ws.on("message", handle_message)
ws.on("search_update", handle_search_update)

# Subscribe to topics
ws.subscribe("messages", "search_updates", "transfer_updates")

# Listen for events (async)
while ws.is_connected():
    await asyncio.sleep(1)

# Cleanup
await client.disconnect_ws()
```

#### Context Manager

```python
# Auto-cleanup with context manager
async with SlskrClient("http://127.0.0.1:5030", "token") as client:
    health = await client.health()
    searches = await client.list_searches()
    # Auto-closed when exiting
```

#### Error Handling

```python
from slskr import ApiError, NetworkError, TimeoutError

try:
    search = await client.create_search("query")
except ApiError as e:
    if e.is_not_found():
        print("Endpoint not found")
    elif e.is_unauthorized():
        print("Invalid API key")
    elif e.is_server_error():
        print(f"Server error: {e.status}")
except TimeoutError:
    print("Request timeout")
except NetworkError as e:
    print(f"Network error: {e}")
```

### Examples

- `examples/basic_usage.py` - Essential API operations
- `examples/batch_operations.py` - Batch operation patterns
- `examples/websocket_events.py` - Real-time event handling
- `examples/advanced_usage.py` - Error handling, retries, concurrency, pagination
- `examples/integration_example.py` - Coordinated multi-feature operations

### API Reference

**SlskrClient**

```python
SlskrClient(
    base_url: str,              # API server URL
    token: str,                 # API authentication token
    timeout: int = 30,          # Request timeout in seconds
    retries: int = 3,           # Automatic retry count
    retry_delay: int = 1,       # Delay between retries
    debug: bool = False         # Enable debug logging
)
```

**Methods**: All are async/await

- `health()` → dict
- `version()` → dict
- `get_config()` → dict
- `get_stats()` → dict
- `list_searches(limit, offset)` → list
- `create_search(query, room, target)` → dict
- `get_search_details(search_id, limit)` → dict
- `list_messages(limit, offset)` → list
- `get_user_messages(username, limit)` → list
- `send_message(recipient, content)` → dict
- `acknowledge_message(message_id)` → None
- `list_transfers(direction, status, limit, offset)` → list
- `create_transfer(direction, peer_username, filename)` → dict
- `get_transfer(transfer_id)` → dict
- `cancel_transfer(transfer_id)` → None
- `connect_ws()` → WebSocketClient
- `disconnect_ws()` → None
- `close()` → None

## slskd Automation Compatibility

Existing automations that use the Python `slskd_api` package can point at
`slskr` with the daemon base URL and API token. The compatibility surface covers
the slskd-style application, session, server, search, transfer, room,
conversation, user, share, file, relay, options, event, log, and telemetry
calls exercised by the local smoke suite.

```python
from slskd_api import SlskdClient

client = SlskdClient(
    host="http://127.0.0.1:5030",
    api_key="your-api-token",
)

print(client.application.state())
print(client.searches.get_all())
```

Run the local compatibility smoke against a temporary authenticated daemon:

```bash
SLSKR_SLSKD_API_SMOKE_TOKEN=slskd-api-smoke-token scripts/run-slskd-api-compat-smoke.sh
```

Useful overrides:

- `SLSKR_SLSKD_API_SMOKE_TOKEN`: required bearer token for the temporary authenticated daemon.
- `SLSKD_API_VERSION`: Python `slskd-api` package version, default `0.2.4`.
- `SLSKD_API_PYTHONPATH`: preinstalled package directory to reuse instead of installing.
- `SLSKR_SLSKD_API_SMOKE_PORT`: fixed local daemon port.
- `SLSKR_SLSKD_API_SMOKE_DIR`: fixed temporary state/log directory.

## Go Client

### Installation

```bash
# Add to go.mod
require github.com/snapetech/slskr/client-go v0.1.0

# Or copy client files
cp -r client-go/* your-project/
```

### Quick Start

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
        "your-api-key-here",
    )
    ctx := context.Background()
    
    // Check server health
    health, err := client.Health(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Server status: %v\n", health["status"])
    
    // Create search
    search, err := client.CreateSearch(ctx, "beethoven symphony")
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Search ID: %v\n", search["id"])
    
    // List transfers
    transfers, err := client.ListTransfers(ctx, "download", "active", 10, 0)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Active transfers: %d\n", len(transfers))
}
```

### Features

#### Core API Methods

```go
// Health & Version
client.Health(ctx)
client.Version(ctx)

// Configuration
client.GetConfig(ctx)
client.GetStats(ctx)
client.GetCapabilities(ctx)

// Search
client.ListSearches(ctx, limit, offset)
client.CreateSearch(ctx, query)

// Messages
client.ListMessages(ctx, limit, offset)
client.GetUserMessages(ctx, username, limit)
client.SendMessage(ctx, recipient, content)
client.AcknowledgeMessage(ctx, messageID)

// Transfers
client.ListTransfers(ctx, direction, status, limit, offset)
client.SendMessage(ctx, recipient, content)

// Users & Rooms
client.GetUser(ctx, username)
client.ListUsers(ctx, limit, offset)
client.ListRooms(ctx)
client.GetRoom(ctx, roomID)
client.JoinRoom(ctx, roomName)
client.LeaveRoom(ctx, roomID)

// Shares
client.ListShares(ctx, limit, offset)
client.RefreshShares(ctx)

// Filters
client.GetFilters(ctx)
client.UpdateFilters(ctx, filters)
```

#### Batch Operations

```go
batch := client.NewBatchBuilder()

stats := "stats"
config := "config"
caps := "caps"

batch.Get("/api/stats", &stats)
batch.Get("/api/config", &config)
batch.Get("/api/capabilities", &caps)

batch.Post("/api/searches", map[string]interface{}{
    "query": "search term",
}, nil)

batch.Delete("/api/transfers/123", nil)

response, err := batch.Execute(ctx)
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Successful: %d\n", len(response.GetSuccessful()))
fmt.Printf("Failed: %d\n", len(response.GetFailed()))
fmt.Printf("Total time: %dms\n", response.TotalTimeMs)
```

#### WebSocket Events

```go
ws := client.NewWebSocketClient(true)
err := ws.Connect(ctx)
if err != nil {
    log.Fatal(err)
}
defer ws.Disconnect(ctx)

// Create event channels
messageCh := make(chan interface{}, 100)
connectionCh := make(chan bool, 10)
errorCh := make(chan error, 10)

// Register listeners
ws.On("message", messageCh)
ws.OnConnectionChange(connectionCh)
ws.OnError(errorCh)

// Subscribe to topics
ws.Subscribe("messages", "search_updates", "transfer_updates")

// Listen for events
for {
    select {
    case event := <-messageCh:
        fmt.Printf("Message: %v\n", event)
    case connected := <-connectionCh:
        fmt.Printf("Connected: %v\n", connected)
    case err := <-errorCh:
        fmt.Printf("Error: %v\n", err)
    case <-time.After(10 * time.Second):
        return
    }
}
```

#### Concurrency

```go
var wg sync.WaitGroup

queries := []string{"bach", "mozart", "beethoven"}
results := make(chan map[string]interface{}, len(queries))

for _, q := range queries {
    wg.Add(1)
    go func(query string) {
        defer wg.Done()
        search, err := client.CreateSearch(ctx, query)
        if err != nil {
            return
        }
        results <- search
    }(q)
}

go func() {
    wg.Wait()
    close(results)
}()

for result := range results {
    fmt.Printf("Search created: %v\n", result["id"])
}
```

### Examples

- `examples/basic_usage.go` - Essential API operations
- `examples/batch_operations.go` - Batch operation patterns
- `examples/websocket_events.go` - Real-time event handling
- `examples/advanced_usage.go` - Concurrency, pagination, error handling, integration

### API Reference

**NewClient**

```go
NewClient(baseURL, token string) *Client
```

**Client Fields**

```go
type Client struct {
    BaseURL    string
    Token      string
    HTTPClient *http.Client
    Timeout    time.Duration
}
```

**Batch Methods**

```go
builder := client.NewBatchBuilder()
builder.Get(path, opID)
builder.Post(path, body, opID)
builder.Put(path, body, opID)
builder.Delete(path, opID)
builder.Size() int
builder.Clear()
response, err := builder.Execute(ctx)
```

## TypeScript/JavaScript Client

See [client-ts/README.md](../client-ts/README.md) for complete documentation of
the TypeScript client.

### Quick Installation

```bash
npm install @slskr/api-client
# or
yarn add @slskr/api-client
```

### Quick Usage

```typescript
import SlskrClient from '@slskr/api-client';

const client = new SlskrClient({
    baseUrl: 'http://127.0.0.1:5030',
    token: 'your-api-key'
});

// Create search
const search = await client.search.create({
    query: 'beethoven symphony'
});

// Get results
const results = await client.search.getResults(search.id);

// Send message
await client.messages.send({
    recipient: 'username',
    content: 'Hello!'
});
```

## Common Patterns

### Retry Logic

**Python**

```python
for attempt in range(3):
    try:
        result = await client.create_search(query)
        return result
    except TimeoutError:
        if attempt < 2:
            await asyncio.sleep(2 ** attempt)
```

**Go**

```go
for attempt := 0; attempt < 3; attempt++ {
    result, err := client.CreateSearch(ctx, query)
    if err == nil {
        return result, nil
    }
    if attempt < 2 {
        time.Sleep(time.Duration(math.Pow(2, float64(attempt))) * time.Second)
    }
}
```

### Pagination

**Python**

```python
limit = 50
offset = 0
all_results = []

while True:
    results = await client.list_searches(limit=limit, offset=offset)
    if not results:
        break
    all_results.extend(results)
    if len(results) < limit:
        break
    offset += limit
```

**Go**

```go
limit := 50
offset := 0
var allResults []map[string]interface{}

for {
    results, err := client.ListSearches(ctx, limit, offset)
    if err != nil || len(results) == 0 {
        break
    }
    allResults = append(allResults, results...)
    if len(results) < limit {
        break
    }
    offset += limit
}
```

### Concurrent Operations

**Python**

```python
tasks = [client.create_search(q) for q in queries]
results = await asyncio.gather(*tasks, return_exceptions=True)
```

**Go**

```go
results := make(chan map[string]interface{}, len(queries))
var wg sync.WaitGroup

for _, q := range queries {
    wg.Add(1)
    go func(query string) {
        defer wg.Done()
        result, _ := client.CreateSearch(ctx, query)
        results <- result
    }(q)
}
```

## Error Handling

### Python

```python
from slskr import ApiError, NetworkError, TimeoutError

try:
    await client.create_search(query)
except ApiError as e:
    if e.is_unauthorized():
        # 401 - Invalid token
    elif e.is_not_found():
        # 404 - Endpoint not found
    elif e.is_server_error():
        # 5xx - Server error
except TimeoutError:
    # Request timed out
except NetworkError as e:
    # Network connectivity issue
```

### Go

```go
import "net/http"

result, err := client.CreateSearch(ctx, query)
if err != nil {
    // Handle based on error type
    if strings.Contains(err.Error(), "401") {
        // Unauthorized
    } else if strings.Contains(err.Error(), "404") {
        // Not found
    } else if strings.Contains(err.Error(), "5") {
        // Server error
    }
}
```

## Performance Considerations

### Batch Operations

Use batch operations to reduce request overhead:

```python
# Inefficient: 3 separate requests
await client.get_stats()
await client.get_config()
await client.get_capabilities()

# Efficient: 1 batch request
batch = client.batch.builder()
batch.get("/api/stats")
batch.get("/api/config")
batch.get("/api/capabilities")
response = await batch.execute()
```

### Connection Pooling

Python automatically pools connections via aiohttp. Go's http.Client also pools connections.

### Rate Limiting

Respect API rate limits:

```python
# Automatic retry with exponential backoff
client = SlskrClient(..., retries=3, retry_delay=1)

# Manual rate limiting
import asyncio
semaphore = asyncio.Semaphore(5)

async def limited_search(query):
    async with semaphore:
        return await client.create_search(query)
```

## Testing

### Python

```bash
cd client-python
python -m pytest examples/
```

### Go

```bash
cd client-go
go test ./...
go run examples/basic_usage.go
```

## Contributing

Client libraries are maintained in:
- `client-python/` - Python async client
- `client-go/` - Go client
- `client-ts/` - TypeScript/JavaScript client

## Support

- **Issues**: GitHub Issues
- **Documentation**: See `docs/` directory
- **Examples**: See `examples/` directories in each client
- **API Reference**: See `docs/openapi.json`

## License

Same as slskr main project
