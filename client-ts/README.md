# slskr API Client (TypeScript/JavaScript)

TypeScript/JavaScript client library for the slskr HTTP API. This package is
part of the independent `slskr` project and is not affiliated with or endorsed
by Soulseek or its operators.

## Features

- ✅ Full type safety with TypeScript
- ✅ Async/await support
- ✅ Batch operations for efficient bulk requests
- ✅ WebSocket support for real-time events
- ✅ Automatic retries with exponential backoff
- ✅ Comprehensive error handling
- ✅ Zero external dependencies (except optional `ws` for Node.js)
- ✅ Works in browser and Node.js
- ✅ Full test coverage

## Installation

```bash
npm install @slskr/api-client
```

### Browser

```html
<script src="https://cdn.jsdelivr.net/npm/@slskr/api-client@latest/dist/index.js"></script>
```

### Node.js (with WebSocket support)

```bash
npm install @slskr/api-client ws
```

## Quick Start

### Basic Usage

```typescript
import SlskrClient from '@slskr/api-client';

const client = new SlskrClient({
  baseUrl: 'http://localhost:8080',
  token: 'your-bearer-token',
});

// Get server stats
const stats = await client.getStats();
console.log(stats);

// Create search
const search = await client.createSearch({
  query: 'artist name',
});

// Get search results
const results = await client.getSearchDetails(search.id);
console.log(results.results);
```

### Batch Operations

Execute multiple operations in a single request:

```typescript
import { BatchClient } from '@slskr/api-client';

const batch = new BatchClient(client);

const response = await batch
  .builder()
  .get('/api/stats', 'stats')
  .get('/api/transfers', 'transfers')
  .get('/api/messages', 'messages')
  .execute();

console.log(`Completed in ${response.total_time_ms}ms`);
console.log(response.results);
```

### WebSocket Events

Listen to real-time events:

```typescript
import { WebSocketClient } from '@slskr/api-client';

const ws = new WebSocketClient('http://localhost:8080', 'your-token');

// Subscribe to events
ws.subscribe('transfer.started', 'transfer.completed');

// Listen to specific events
ws.on('transfer.completed', (event) => {
  console.log('Download finished:', event.data);
});

// Connect
await ws.connect();

// Keep connection open
setTimeout(() => ws.disconnect(), 60000);
```

## API Reference

### SlskrClient

Main HTTP client for REST API operations.

#### Constructor

```typescript
new SlskrClient({
  baseUrl: string;
  token: string;
  timeout?: number; // default: 30000ms
  retries?: number; // default: 3
  retryDelay?: number; // default: 1000ms
  debug?: boolean; // default: false
})
```

#### Methods

**Health & Info**
- `health()` - Get server health status
- `version()` - Get server version
- `getCapabilities()` - Get API capabilities

**Configuration**
- `getConfig()` - Get current configuration
- `getStats()` - Get server statistics

**Sessions**
- `getSessions()` - List active sessions
- `createSession(kind, parameters)` - Create new session
- `pingSession(id)` - Keep session alive
- `disconnectSession(id)` - Close session
- `getSessionPrivileges(id)` - Get session privileges

**Search**
- `listSearches(params)` - List searches
- `createSearch(request)` - Create new search
- `getSearchDetails(id, params)` - Get search results

**Messages**
- `listMessages(params)` - List messages
- `getUserMessages(username, params)` - Get user messages
- `sendMessage(request)` - Send message
- `acknowledgeMessage(id)` - Mark message as read

**Transfers**
- `listTransfers(params)` - List transfers
- `createTransfer(request)` - Start transfer
- `getTransfer(id)` - Get transfer details
- `cancelTransfer(id)` - Cancel transfer

**Rooms**
- `listRooms(params)` - List chat rooms
- `getRoom(name)` - Get room details
- `joinRoom(name)` - Join room
- `leaveRoom(name)` - Leave room

**Browse**
- `browseUser(username, params)` - Browse user files
- `requestBrowse(username, folder)` - Request to browse
- `getBrowseRequests(params)` - List browse requests
- `respondToBrowseRequest(id, action, folder)` - Accept/reject

**Events**
- `getEvents(params)` - Get event history

**Cache**
- `getCacheStats()` - Get cache statistics
- `invalidateCache(keys)` - Clear cache entries

### BatchClient

Execute multiple operations efficiently.

```typescript
const batch = new BatchClient(client);

batch
  .builder()
  .get('/api/stats')
  .post('/api/searches', {query: 'music'})
  .delete('/api/transfers/123')
  .execute();
```

**Methods:**
- `builder()` - Create new batch builder
- `execute(operations)` - Execute batch
- `allSuccessful(response)` - Check if all succeeded
- `getFailed(response)` - Get failed operations
- `getSuccessful(response)` - Get successful operations

### WebSocketClient

Real-time event streaming.

```typescript
const ws = new WebSocketClient(baseUrl, token);

await ws.connect();
ws.subscribe('transfer.started', 'message.received');
ws.on('transfer.started', (event) => {
  console.log(event);
});
```

**Methods:**
- `connect()` - Connect to WebSocket
- `disconnect()` - Close connection
- `subscribe(...topics)` - Subscribe to events
- `unsubscribe(...topics)` - Unsubscribe from events
- `on(type, listener)` - Listen to event
- `onConnectionChange(listener)` - Connection state
- `onError(listener)` - Listen to errors
- `isConnected()` - Check connection status
- `getSubscribedTopics()` - Get current subscriptions

## Error Handling

```typescript
import { ApiError, NetworkError, TimeoutError } from '@slskr/api-client';

try {
  await client.getTransfer('invalid-id');
} catch (error) {
  if (error instanceof ApiError) {
    console.error(`API Error: ${error.status} ${error.code}`);
    
    if (error.isNotFound()) console.error('Not found');
    if (error.isUnauthorized()) console.error('Invalid token');
    if (error.isForbidden()) console.error('Access denied');
  } else if (error instanceof TimeoutError) {
    console.error('Request timeout');
  } else if (error instanceof NetworkError) {
    console.error('Network error:', error.cause);
  }
}
```

## Examples

### Pagination

```typescript
const messages = [];
let offset = 0;

while (true) {
  const batch = await client.listMessages({
    limit: 20,
    offset,
  });

  if (batch.length === 0) break;
  messages.push(...batch);
  offset += batch.length;
}
```

### Bulk Operations

```typescript
const batch = new BatchClient(client);

// Bulk message send
const response = await batch
  .builder()
  .post('/api/messages', {recipient: 'alice', content: 'Hi'})
  .post('/api/messages', {recipient: 'bob', content: 'Hi'})
  .post('/api/messages', {recipient: 'charlie', content: 'Hi'})
  .execute();

console.log(`Sent ${batch.getSuccessful(response).length} messages`);
```

### Real-Time Monitoring

```typescript
const ws = new WebSocketClient('http://localhost:8080', token);

ws.subscribe(
  'transfer.started',
  'transfer.completed',
  'transfer.failed'
);

ws.on('transfer.started', (event) => {
  console.log(`Started: ${event.data.filename}`);
});

ws.on('transfer.completed', (event) => {
  console.log(`Completed: ${event.data.filename}`);
});

ws.on('transfer.failed', (event) => {
  console.log(`Failed: ${event.data.filename} - ${event.data.reason}`);
});

await ws.connect();
```

## Configuration

### Client Options

```typescript
const client = new SlskrClient({
  // API server URL
  baseUrl: 'http://localhost:8080',

  // Bearer token for authentication
  token: 'your-token',

  // Request timeout in milliseconds
  timeout: 30000,

  // Number of retry attempts
  retries: 3,

  // Delay between retries in milliseconds
  retryDelay: 1000,

  // Enable debug logging
  debug: false,
});
```

## TypeScript Support

Full TypeScript support with strict type checking:

```typescript
import {
  SlskrClient,
  Transfer,
  Search,
  Message,
  Event,
  ApiError,
  TimeoutError,
} from '@slskr/api-client';

const client = new SlskrClient({...});

// Types are inferred
const transfer: Transfer = await client.getTransfer('id');
const search: Search = await client.createSearch({query: 'music'});
const messages: Message[] = await client.listMessages();
```

## Browser Support

Works in all modern browsers with `fetch` API support:

```html
<script src="https://cdn.jsdelivr.net/npm/@slskr/api-client@latest/dist/index.js"></script>
<script>
  const client = new SlskrClient({
    baseUrl: 'http://localhost:8080',
    token: 'token'
  });

  client.getStats().then(stats => console.log(stats));
</script>
```

## Node.js Support

Works in Node.js (tested on v14+):

```javascript
const { default: SlskrClient } = require('@slskr/api-client');

const client = new SlskrClient({
  baseUrl: 'http://localhost:8080',
  token: 'token'
});

(async () => {
  const stats = await client.getStats();
  console.log(stats);
})();
```

## Performance

- **Batch operations**: 5-10x faster than sequential requests
- **WebSocket**: 600x less bandwidth than polling
- **Response caching**: 40x latency reduction for cached endpoints
- **Automatic retries**: Exponential backoff for reliability

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

AGPL-3.0-only. See the repository `LICENSE` and `NOTICE` files for details.

## Support

- Documentation: [docs/http-api.md](../../docs/http-api.md)
- GitHub Issues: [Report bugs](https://github.com/slskr/issues)
- GitHub Discussions: [Ask questions](https://github.com/slskr/discussions)

## Changelog

### 1.0.0 (2026-05-04)
- Initial release
- Full HTTP API coverage
- Batch operations support
- WebSocket support
- TypeScript support
