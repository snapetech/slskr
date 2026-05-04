# soulseekR Python API Client

Official Python async client library for the soulseekR HTTP API.

## Features

- ✅ Full async/await support
- ✅ Complete HTTP API coverage
- ✅ Batch operations
- ✅ WebSocket real-time events
- ✅ Automatic retries
- ✅ Comprehensive error handling
- ✅ Type hints

## Installation

```bash
pip install soulseekr-api-client
```

## Quick Start

```python
import asyncio
from soulseekr import SoulseekrClient

async def main():
    async with SoulseekrClient(
        base_url="http://localhost:8080",
        token="your-bearer-token"
    ) as client:
        # Get stats
        stats = await client.get_stats()
        print(stats)

        # Create search
        search = await client.create_search(query="artist name")
        print(f"Search ID: {search['id']}")

        # Get search results
        results = await client.get_search_details(search['id'])
        print(f"Found {len(results['results'])} results")

asyncio.run(main())
```

## Examples

### Batch Operations

```python
from soulseekr import BatchClient

async def batch_example(client):
    batch = BatchClient(client)

    response = await batch.builder() \
        .get("/api/stats", "stats") \
        .get("/api/transfers", "transfers") \
        .get("/api/messages", "messages") \
        .execute()

    print(f"Completed in {response.total_time_ms}ms")
    print(f"Successful: {len(response.get_successful())}")
    print(f"Failed: {len(response.get_failed())}")
```

### WebSocket Events

```python
from soulseekr import WebSocketClient

async def websocket_example():
    ws = WebSocketClient(
        base_url="http://localhost:8080",
        token="your-token"
    )

    # Listen to transfer events
    @ws.on("transfer.completed")
    async def on_transfer_completed(event):
        print(f"Transfer completed: {event['data']}")

    # Connect
    await ws.connect()

    # Subscribe to events
    ws.subscribe("transfer.started", "transfer.completed", "transfer.failed")

    # Keep connection open
    try:
        while True:
            await asyncio.sleep(1)
    finally:
        await ws.disconnect()
```

### Error Handling

```python
from soulseekr import SoulseekrClient, ApiError, TimeoutError

async def error_handling():
    async with SoulseekrClient(...) as client:
        try:
            await client.get_transfer("invalid-id")
        except ApiError as e:
            if e.is_not_found():
                print("Transfer not found")
            elif e.is_unauthorized():
                print("Invalid token")
            elif e.is_conflict():
                print("Transfer conflict")
            else:
                print(f"API error: {e.code}")
        except TimeoutError:
            print("Request timed out")
```

## API Reference

### SoulseekrClient

Main HTTP client for REST API operations.

#### Methods

**Health & Info**
- `health()` - Server health
- `version()` - Version info
- `get_capabilities()` - API capabilities
- `get_config()` - Configuration
- `get_stats()` - Statistics

**Search**
- `list_searches(limit, offset)` - List searches
- `create_search(query, room, target)` - Create search
- `get_search_details(search_id)` - Search details

**Messages**
- `list_messages(limit, offset)` - List messages
- `get_user_messages(username)` - User messages
- `send_message(recipient, content)` - Send message
- `acknowledge_message(message_id)` - Mark read

**Transfers**
- `list_transfers(direction, status)` - List transfers
- `create_transfer(direction, peer, filename)` - Start transfer
- `get_transfer(transfer_id)` - Transfer details
- `cancel_transfer(transfer_id)` - Cancel transfer

### BatchClient

Execute batch operations efficiently.

```python
batch = BatchClient(client)
response = await batch.builder() \
    .get("/api/stats") \
    .post("/api/searches", {"query": "music"}) \
    .delete("/api/transfers/123") \
    .execute()
```

### WebSocketClient

Real-time event streaming.

```python
ws = WebSocketClient(base_url, token)
await ws.connect()
ws.subscribe("transfer.started", "transfer.completed")

@ws.on("transfer.completed")
async def on_complete(event):
    print(event)
```

## Configuration

```python
client = SoulseekrClient(
    base_url="http://localhost:8080",
    token="your-token",
    timeout=30,  # Request timeout in seconds
    retries=3,   # Retry attempts
    retry_delay=1,  # Delay between retries
    debug=False  # Enable debug logging
)
```

## Contributing

Contributions welcome! Please see CONTRIBUTING.md

## License

MIT - See LICENSE for details
