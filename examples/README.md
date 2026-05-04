# soulseekR API Examples

This directory contains example applications demonstrating how to use the soulseekR HTTP API.

## Examples

### 1. Basic REST API Usage (`basic-usage.ts`)

Simple examples showing how to use the HTTP client to:
- Check server health
- Get server version and stats
- List and create searches
- Send messages
- Manage transfers

**Usage:**
```bash
cd client-ts
npm install
npm run build
node examples/basic-usage.js
```

### 2. Search Monitor

Real-time search monitoring application:
- Listen to search events via WebSocket
- Display active searches
- Track search results
- Show search statistics

**Python Example:**
```python
import asyncio
import websockets
import json

async def monitor_searches():
    uri = "ws://localhost:8080/api/events/ws"
    headers = {"Authorization": "Bearer YOUR-TOKEN"}
    
    async with websockets.connect(uri, subprotocols=["chat"]) as ws:
        # Subscribe to search events
        await ws.send(json.dumps({
            "type": "subscribe",
            "topics": ["search.started", "search.completed"]
        }))
        
        async for msg in ws:
            event = json.loads(msg)
            if event["type"] == "search.started":
                print(f"Search started: {event['data']['query']}")
            elif event["type"] == "search.completed":
                print(f"Search completed: {event['data']['id']}")

asyncio.run(monitor_searches())
```

### 3. Transfer Manager

Bulk transfer management:
- Start multiple transfers
- Monitor transfer progress
- Cancel transfers
- Get transfer statistics

**Bash Example:**
```bash
#!/bin/bash

TOKEN="your-bearer-token"
API="http://localhost:8080"

# List active downloads
curl -H "Authorization: Bearer $TOKEN" \
     "$API/api/transfers?direction=download&status=active"

# Start multiple downloads (batch)
curl -X POST -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -H "Origin: http://localhost:8080" \
     -d '{
       "operations": [
         {"id":"d1","method":"POST","path":"/api/transfers","body":{"direction":"download","peer_username":"user1","filename":"song.mp3"}},
         {"id":"d2","method":"POST","path":"/api/transfers","body":{"direction":"download","peer_username":"user2","filename":"album.zip"}}
       ]
     }' \
     "$API/api/batch"

# Get transfer statistics
curl -H "Authorization: Bearer $TOKEN" \
     "$API/api/stats" | jq '.transfers'
```

### 4. Message Broadcaster

Send messages to multiple users:
- Load recipient list
- Send bulk messages
- Track delivery status
- Handle errors

**Node.js Example:**
```javascript
const SoulseekrClient = require('@soulseekr/api-client');

const client = new SoulseekrClient({
  baseUrl: 'http://localhost:8080',
  token: 'your-token'
});

const recipients = ['user1', 'user2', 'user3'];

async function broadcastMessage(message) {
  for (const recipient of recipients) {
    try {
      const msg = await client.sendMessage({
        recipient,
        content: message
      });
      console.log(`Sent to ${recipient}: ${msg.id}`);
    } catch (error) {
      console.error(`Failed to send to ${recipient}:`, error.message);
    }
  }
}

broadcastMessage('Hello everyone!');
```

### 5. File Browser

Browse shared files from other users:
- List user files
- Search for files
- Track browse requests
- Display folder structure

**Curl Example:**
```bash
#!/bin/bash

TOKEN="your-bearer-token"
API="http://localhost:8080"

# Browse user files
curl -H "Authorization: Bearer $TOKEN" \
     "$API/api/browse/username"

# Browse specific folder
curl -H "Authorization: Bearer $TOKEN" \
     "$API/api/browse/username?folder=Music"

# Request browse permission
curl -X POST -H "Authorization: Bearer $TOKEN" \
     -H "Origin: http://localhost:8080" \
     -H "Content-Type: application/json" \
     -d '{"folder":null}' \
     "$API/api/browse/username"

# Accept browse request
curl -X POST -H "Authorization: Bearer $TOKEN" \
     -H "Origin: http://localhost:8080" \
     -H "Content-Type: application/json" \
     -d '{"action":"accept","folder":"/music"}' \
     "$API/api/browse/requests/request-id"
```

### 6. Dashboard

Web dashboard showing:
- Real-time transfer progress
- Active searches
- Message statistics
- Server health
- Performance metrics

**HTML Example:**
```html
<!DOCTYPE html>
<html>
<head>
  <title>soulseekR Dashboard</title>
  <script src="https://cdn.jsdelivr.net/npm/@soulseekr/api-client"></script>
</head>
<body>
  <h1>soulseekR Dashboard</h1>
  
  <div id="stats"></div>
  <div id="transfers"></div>
  <div id="searches"></div>

  <script>
    const client = new SoulseekrClient({
      baseUrl: 'http://localhost:8080',
      token: 'your-token'
    });

    async function updateDashboard() {
      const stats = await client.getStats();
      document.getElementById('stats').innerHTML = JSON.stringify(stats, null, 2);

      const transfers = await client.listTransfers();
      document.getElementById('transfers').innerHTML = 
        transfers.map(t => `${t.filename}: ${t.progress_percent}%`).join('<br>');

      const searches = await client.listSearches();
      document.getElementById('searches').innerHTML = 
        searches.map(s => `${s.query}: ${s.results_count} results`).join('<br>');
    }

    updateDashboard();
    setInterval(updateDashboard, 5000);
  </script>
</body>
</html>
```

### 7. Performance Benchmark

Benchmark API performance:
- Test latency
- Measure throughput
- Compare batch vs sequential
- Profile WebSocket performance

**Node.js Benchmark:**
```javascript
const { default: SoulseekrClient, BatchClient } = require('@soulseekr/api-client');

async function benchmark() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'token'
  });

  // Test sequential requests
  console.time('Sequential (10 requests)');
  for (let i = 0; i < 10; i++) {
    await client.getStats();
  }
  console.timeEnd('Sequential (10 requests)');

  // Test batch requests
  const batch = new BatchClient(client);
  console.time('Batch (10 operations)');
  for (let i = 0; i < 10; i++) {
    batch.builder()
      .get('/api/stats', `stats-${i}`)
      .execute();
  }
  console.timeEnd('Batch (10 operations)');
}

benchmark();
```

### 8. Error Handling

Comprehensive error handling examples:
- Authentication errors
- Network errors
- Timeout handling
- Graceful degradation

**Example:**
```typescript
import {
  SoulseekrClient,
  ApiError,
  NetworkError,
  TimeoutError
} from '@soulseekr/api-client';

async function robustOperation() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'token',
    retries: 3,
    timeout: 30000
  });

  try {
    await client.createTransfer({
      direction: 'download',
      peer_username: 'user',
      filename: 'file.mp3'
    });
  } catch (error) {
    if (error instanceof ApiError) {
      if (error.isNotFound()) {
        console.error('Transfer not found');
      } else if (error.isUnauthorized()) {
        console.error('Invalid token - please authenticate');
      } else if (error.isConflict()) {
        console.error('Transfer already exists');
      }
    } else if (error instanceof TimeoutError) {
      console.error('Request timed out - server may be slow');
    } else if (error instanceof NetworkError) {
      console.error('Network error - check connection:', error.cause);
    }
  }
}
```

## Running Examples

### Prerequisites

- Node.js 14+ (for TypeScript examples)
- Python 3.7+ (for Python examples)
- curl (for Bash examples)
- soulseekR server running on http://localhost:8080

### Setup

1. Set Bearer token:
```bash
export SOULSEEK_TOKEN="your-bearer-token"
```

2. For TypeScript examples:
```bash
cd client-ts
npm install
npm run build
```

3. Run examples:
```bash
# TypeScript
npx ts-node examples/basic-usage.ts

# Python
python3 examples/search_monitor.py

# Bash
bash examples/transfer_manager.sh

# Node.js
node examples/message_broadcaster.js
```

## Integration Examples

### Docker Compose Setup

```yaml
version: '3'
services:
  soulseekr:
    image: soulseekr:latest
    ports:
      - "8080:8080"
    environment:
      - HTTP_API_BEARER_TOKEN=secret-token
      - RUST_LOG=info

  example-app:
    image: node:18
    depends_on:
      - soulseekr
    volumes:
      - ./examples:/app
    working_dir: /app
    command: npm start
    environment:
      - SOULSEEK_API_URL=http://soulseekr:8080
      - SOULSEEK_TOKEN=secret-token
```

### Kubernetes Deployment

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: example-app-config
data:
  SOULSEEK_API_URL: "http://soulseekr:8080"
  SOULSEEK_TOKEN: "secret-token"

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: example-app
spec:
  replicas: 1
  selector:
    matchLabels:
      app: example-app
  template:
    metadata:
      labels:
        app: example-app
    spec:
      containers:
      - name: app
        image: node:18
        envFrom:
        - configMapRef:
            name: example-app-config
        volumeMounts:
        - name: app
          mountPath: /app
      volumes:
      - name: app
        configMap:
          name: example-app-code
```

## Tips & Best Practices

1. **Use batch operations** for multiple requests
2. **Subscribe to WebSocket events** instead of polling
3. **Implement error handling** for production apps
4. **Set reasonable timeouts** for network operations
5. **Use rate limiting headers** to respect server limits
6. **Cache responses** where appropriate
7. **Monitor performance** with built-in metrics
8. **Test with the API** before deployment

## Support

- See [docs/http-api.md](../docs/http-api.md) for API reference
- See [docs/http-api-features.md](../docs/http-api-features.md) for advanced features
- See [client-ts/README.md](../client-ts/README.md) for TypeScript client docs
- Open an issue on GitHub for bugs or feature requests

## License

MIT - See LICENSE for details
