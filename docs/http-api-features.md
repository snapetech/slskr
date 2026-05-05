# slskr HTTP API - Advanced Features Guide

## Overview

This guide documents advanced HTTP API features including logging, batch operations, WebSocket support, and response caching.

## Request/Response Logging

### Overview

slskr includes built-in structured logging for HTTP API requests and responses to help with debugging, monitoring, and performance analysis.

### Configuration

Logging is controlled via the `RUST_LOG` environment variable:

```bash
# Info level (default)
./target/release/slskr

# Debug level (log all requests)
RUST_LOG=debug ./target/release/slskr

# Trace level (maximum detail)
RUST_LOG=trace ./target/release/slskr

# Warn level (errors and warnings only)
RUST_LOG=warn ./target/release/slskr
```

### Log Output Format

Logs are printed to stderr with the following format:

```
[TIMESTAMP] METHOD PATH STATUS bytes in XXms [error]
```

**Example:**
```
[INFO] GET /api/stats 200 256 bytes in 1ms
[WARN] POST /api/searches 400 128 bytes in 5ms - Invalid query
[ERROR] GET /api/transfers 500 64 bytes in 100ms - Database error
```

### Log Levels

| Level | Purpose | Example |
|-------|---------|---------|
| **TRACE** | Maximum detail, includes all requests | Development only |
| **DEBUG** | Request/response details | Debugging issues |
| **INFO** | Normal operations (default) | Production |
| **WARN** | Client errors (4xx) | Monitoring |
| **ERROR** | Server errors (5xx) | Alerting |

### Extracting Logs

Redirect logs to file for analysis:

```bash
./target/release/slskr > api.log 2>&1 &

# Monitor in real-time
tail -f api.log

# Filter specific operations
grep "POST" api.log | tail -20

# Find slow requests
grep -E "in [0-9]{3,}ms" api.log
```

### Log Analysis Tips

Find slow requests:
```bash
grep -oE "in [0-9]+ms" api.log | sort -V | tail -10
```

Count requests by method:
```bash
grep -oE "^\[.*\] [A-Z]+" api.log | cut -d' ' -f2 | sort | uniq -c
```

## Batch Operations

### Overview

Batch endpoints allow you to execute multiple API operations in a single HTTP request, reducing round-trip time and overhead.

### Batch Request Format

**Endpoint:** `POST /api/batch`

**Request Body:**
```json
{
  "operations": [
    {
      "id": "op1",
      "method": "GET",
      "path": "/api/stats"
    },
    {
      "id": "op2",
      "method": "GET",
      "path": "/api/transfers"
    },
    {
      "id": "op3",
      "method": "POST",
      "path": "/api/searches",
      "body": "{\"query\":\"music\"}"
    }
  ]
}
```

### Batch Response Format

**Response:**
```json
{
  "results": [
    {
      "id": "op1",
      "status": 200,
      "body": "{\"total_size\":1000000,...}"
    },
    {
      "id": "op2",
      "status": 200,
      "body": "{\"transfers\":[...]}"
    },
    {
      "id": "op3",
      "status": 201,
      "body": "{\"search_id\":\"search-123\"}"
    }
  ],
  "total_time_ms": 45
}
```

### Use Cases

**1. Get All Stats at Once**

Instead of:
```bash
curl /api/stats
curl /api/transfers
curl /api/messages
```

Use:
```bash
curl -X POST /api/batch -d '{
  "operations": [
    {"id":"s","method":"GET","path":"/api/stats"},
    {"id":"t","method":"GET","path":"/api/transfers"},
    {"id":"m","method":"GET","path":"/api/messages"}
  ]
}'
```

**2. Bulk Message Send**

```json
{
  "operations": [
    {"id":"1","method":"POST","path":"/api/messages","body":"{\"recipient\":\"alice\",\"content\":\"Hi\"}"},
    {"id":"2","method":"POST","path":"/api/messages","body":"{\"recipient\":\"bob\",\"content\":\"Hello\"}"},
    {"id":"3","method":"POST","path":"/api/messages","body":"{\"recipient\":\"charlie\",\"content\":\"Hey\"}"}
  ]
}
```

**3. Cancel Multiple Transfers**

```json
{
  "operations": [
    {"id":"1","method":"DELETE","path":"/api/transfers/123"},
    {"id":"2","method":"DELETE","path":"/api/transfers/124"},
    {"id":"3","method":"DELETE","path":"/api/transfers/125"}
  ]
}
```

### Performance Benefits

- **Reduced Latency**: Single request/response cycle instead of multiple
- **Lower Overhead**: Amortize HTTP header overhead
- **Atomic Operations**: All operations processed in sequence
- **Timing**: See total execution time in response

### Error Handling

Each operation result includes status code. Check individual results:

```javascript
const response = await fetch('/api/batch', {
  method: 'POST',
  headers: {'Authorization': 'Bearer token'},
  body: JSON.stringify({operations})
});

const batch = await response.json();

// Check individual results
batch.results.forEach(result => {
  if (result.status >= 400) {
    console.error(`Operation ${result.id} failed: ${result.status}`);
  }
});
```

## WebSocket Support

### Overview

WebSocket connections enable real-time, bidirectional communication for event streaming, eliminating the need for polling.

### Connection Upgrade

**Endpoint:** `GET /api/events/ws` or `WS /api/events/stream`

**Connection Header:**
```
GET /api/events/ws HTTP/1.1
Host: localhost:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
```

### Event Types

WebSocket sends JSON-formatted events:

```json
{
  "type": "search.started",
  "id": "search-123",
  "data": {"query": "music", "room": null},
  "timestamp": "2025-05-04T12:00:00Z"
}
```

**Supported Event Types:**

| Type | Description |
|------|-------------|
| `search.started` | New search initiated |
| `search.completed` | Search finished |
| `search.result` | New result received |
| `transfer.started` | Transfer started |
| `transfer.completed` | Transfer finished |
| `transfer.failed` | Transfer error |
| `message.received` | Incoming message |
| `connection.status` | Connection status changed |
| `room.joined` | Joined room |
| `room.user_joined` | User joined room |
| `room.user_left` | User left room |

### Topic Subscriptions

Subscribe to specific topics to reduce message volume:

```javascript
const token = sessionStorage.getItem('slskr-token') ?? '';
const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
const ws = new WebSocket('ws://localhost:8080/api/events/ws', protocols);

ws.onopen = () => {
  // Subscribe to only search events
  ws.send(JSON.stringify({
    type: 'subscribe',
    topics: ['search.started', 'search.completed']
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(`Event: ${msg.type}`, msg);
};
```

### Example: Node.js Client

```javascript
const WebSocket = require('ws');

const token = process.env.SLSKR_API_TOKEN || '';
const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
const ws = new WebSocket('ws://localhost:8080/api/events/ws', protocols);

ws.on('open', () => {
  console.log('Connected to WebSocket');
  
  // Subscribe to transfers
  ws.send(JSON.stringify({
    type: 'subscribe',
    topics: ['transfer.started', 'transfer.completed']
  }));
});

ws.on('message', (data) => {
  const event = JSON.parse(data);
  
  if (event.type === 'transfer.started') {
    console.log(`Transfer started: ${event.data.filename}`);
  } else if (event.type === 'transfer.completed') {
    console.log(`Transfer completed: ${event.data.filename}`);
  }
});

ws.on('error', (error) => {
  console.error('WebSocket error:', error);
});

ws.on('close', () => {
  console.log('WebSocket closed');
});
```

### Ping/Pong for Keep-Alive

WebSocket server sends periodic pings to detect stale connections:

```javascript
ws.on('ping', () => {
  ws.pong(); // Automatically handled by most libraries
});
```

## Response Caching

### Overview

Static endpoints have configurable caching with TTL to reduce database/processing overhead and improve response times.

### Cached Endpoints

| Endpoint | Default TTL | Rationale |
|----------|-------------|-----------|
| `/api/version` | 1 hour | Never changes |
| `/api/capabilities` | 1 hour | Static capabilities |
| `/api/config` | 5 minutes | Configuration changes rarely |
| `/api/stats` | 1 minute | Regular updates |
| `/api/events` | 30 seconds | Needs freshness |

### Cache Statistics

Access cache metrics:

```bash
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/cache/stats
```

**Response:**
```json
{
  "hits": 1543,
  "misses": 257,
  "evictions": 12,
  "total_requests": 1800,
  "hit_rate": 85.7,
  "entries": 23,
  "max_size": 1000
}
```

### Cache Configuration

Configure cache behavior in `slskr.config.toml`:

```toml
# Cache TTLs
cache_version_ttl_seconds = 3600
cache_capabilities_ttl_seconds = 3600
cache_config_ttl_seconds = 300
cache_stats_ttl_seconds = 60
cache_events_ttl_seconds = 30

# Cache size limits
cache_max_entries = 1000
cache_cleanup_interval_seconds = 300
```

### Cache Invalidation

Caches are automatically invalidated when:
- Entry TTL expires
- Maximum cache size reached (LRU eviction)
- Specific endpoint mutation occurs

**Manual Invalidation:**

```bash
curl -X POST -H "Authorization: Bearer token" \
     http://localhost:8080/api/cache/invalidate \
     -d '{"keys": ["/api/stats", "/api/config"]}'
```

### Performance Impact

With 85% cache hit rate:
- **Original Latency**: ~100ms per request
- **Cached Latency**: ~5ms per cached request
- **Total Improvement**: 94% latency reduction for cached endpoints

## Monitoring & Observability

### Health Check with Details

```bash
# Basic health check
curl http://localhost:8080/api/health

# Get version
curl http://localhost:8080/api/version

# Get capabilities
curl http://localhost:8080/api/capabilities

# Get current stats
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/stats
```

### Metrics Endpoint

```bash
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/metrics
```

**Response:**
```json
{
  "uptime_seconds": 3600,
  "requests_total": 15847,
  "requests_per_second": 4.4,
  "average_latency_ms": 45,
  "p95_latency_ms": 120,
  "p99_latency_ms": 250,
  "active_connections": 23,
  "cache_hit_rate": 85.7
}
```

### Logging for Observability

Enable debug logging for detailed request/response tracking:

```bash
RUST_LOG=debug ./target/release/slskr 2>&1 | tee api.log
```

Parse logs for monitoring:

```bash
# Requests per second
tail -100 api.log | grep -c "^\[.*\]"

# Average response time
tail -100 api.log | grep -oE "in [0-9]+ms" | \
  awk '{sum+=$2; count++} END {print sum/count}'

# Error rate
tail -100 api.log | grep -E "[45][0-9]{2}" | wc -l
```

## Performance Optimization

### Batch Operations vs Sequential

**Sequential (6 requests, ~500ms):**
```bash
time for i in {1..6}; do curl /api/endpoint$i; done
# ~500ms total (80ms each)
```

**Batch (1 request, ~100ms):**
```bash
time curl -X POST /api/batch -d '{"operations":[...]}'
# ~100ms total
```

**Improvement: 5x faster**

### WebSocket vs Polling

**Polling (every 5 seconds, continuous traffic):**
```
HTTP GET /api/events - 5 req/min * 60 min = 300 req/hour
Bandwidth: 300 * 2KB = 600KB/hour
```

**WebSocket (event-driven):**
```
1 connection = 0 KB when idle
Bandwidth: ~100 bytes per event
Average events: 10/hour = 1KB/hour
```

**Improvement: 600x less bandwidth**

### Caching Benefits

**Without caching:**
```
/api/stats: 100ms * 1000 req/min = 100s CPU/min
```

**With 85% cache hit rate:**
```
/api/stats: (15 * 100ms + 85 * 5ms) = 2.425s CPU/min
```

**Improvement: 41x faster aggregated**

## Best Practices

### 1. Use Batch Operations for Bulk Tasks

```javascript
// ❌ Bad: 10 sequential requests
for (const transfer of transfers) {
  await fetch(`/api/transfers/${transfer.id}`, {method: 'DELETE'});
}

// ✅ Good: Single batch request
const operations = transfers.map(t => ({
  id: t.id,
  method: 'DELETE',
  path: `/api/transfers/${t.id}`
}));
await fetch('/api/batch', {
  method: 'POST',
  body: JSON.stringify({operations})
});
```

### 2. Use WebSocket for Real-Time Updates

```javascript
// ❌ Bad: Polling every 5 seconds
setInterval(() => fetch('/api/transfers').then(...), 5000);

// ✅ Good: WebSocket events
const token = sessionStorage.getItem('slskr-token') ?? '';
const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
const ws = new WebSocket('ws://localhost:8080/api/events/ws', protocols);
ws.onmessage = (evt) => {
  if (JSON.parse(evt.data).type === 'transfer.completed') {
    updateUI();
  }
};
```

### 3. Monitor Cache Hit Rate

```bash
# Check cache performance
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/cache/stats | jq '.hit_rate'

# If < 70%, consider increasing TTLs
```

### 4. Enable Logging in Production

```bash
# Low overhead: INFO level
RUST_LOG=info ./target/release/slskr > /var/log/slskr.log 2>&1 &

# Monitor warnings/errors
tail -f /var/log/slskr.log | grep -E "WARN|ERROR"
```

## Troubleshooting

### WebSocket Connection Fails

**Error:** `WebSocket connection failed`

**Solutions:**
1. Check if server supports WebSocket (may be behind reverse proxy)
2. Verify authentication header is present
3. Check firewall/proxy settings for WebSocket support

### Batch Operation Timeout

**Error:** `Request timeout`

**Solutions:**
1. Reduce batch size (max 100 operations recommended)
2. Increase server timeout in reverse proxy
3. Use sequential operations for very large batches

### Cache Hit Rate Low

**Error:** `Hit rate < 50%`

**Solutions:**
1. Increase TTL for static endpoints
2. Check if endpoints are being modified frequently
3. Monitor with: `curl /api/cache/stats`

## References

- [HTTP API Documentation](http-api.md)
- [Deployment Guide](http-api-deployment.md)
- [Performance Analysis](performance-analysis.md)
- [RFC 6455 - WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- [JSON Batch Request/Response RFC](https://jsonapi.org/ext/jsonpatch/)
