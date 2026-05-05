# Webhook API Documentation

slskr now supports webhooks for receiving notifications about important events in real-time. This document describes how to use the webhook API.

## Overview

Webhooks allow you to register HTTP endpoints that will be notified when specific events occur in slskr. Each webhook:
- Has a unique ID for identification
- Targets a specific URL (your server endpoint)
- Subscribes to one or more event types
- Includes an HMAC-SHA256 signature for security verification
- Supports configurable retry policies and timeouts

## Authentication

All webhook endpoints require Bearer token authentication (same as other API endpoints):

```bash
curl -H "Authorization: Bearer your-api-key" https://localhost:5030/api/webhooks
```

## Webhook Events

The following events can trigger webhooks:

### Search Events
- `search.created` - A new search has been initiated
- `search.completed` - A search has completed and results are available

### Transfer Events
- `transfer.started` - A file transfer has started
- `transfer.completed` - A file transfer has completed successfully
- `transfer.failed` - A file transfer has failed

### Message Events
- `message.sent` - An outbound message has been sent
- `message.received` - An inbound message has been received

### User Events
- `user.connected` - A user has connected
- `user.disconnected` - A user has disconnected

### Room Events
- `room.joined` - Successfully joined a chat room
- `room.left` - Left a chat room

### API Key Events
- `apikey.created` - A new API key has been created
- `apikey.revoked` - An API key has been revoked

### Configuration Events
- `config.changed` - The server configuration has changed

## API Endpoints

### Register a Webhook

**Endpoint:** `POST /api/webhooks`

**Request:**
```json
{
  "url": "https://your-server.com/webhook",
  "events": "search.created,search.completed,transfer.completed",
  "secret": "optional-custom-secret"
}
```

**Response:**
```json
{
  "id": "hook_123",
  "secret": "secret_abc123...",
  "status": "created"
}
```

### List All Webhooks

**Endpoint:** `GET /api/webhooks`

**Response:**
```json
{
  "webhooks": [
    {
      "id": "hook_123",
      "url": "https://your-server.com/webhook",
      "events": ["search.created", "search.completed", "transfer.completed"],
      "active": true,
      "created_at": 1714870800,
      "last_triggered": 1714871200,
      "retry_count": 0,
      "max_retries": 3,
      "timeout_seconds": 30
    }
  ]
}
```

### Get Webhook Details

**Endpoint:** `GET /api/webhooks/{webhook_id}`

Returns the same structure as a single webhook from the list endpoint.

### Update Webhook

**Endpoint:** `PATCH /api/webhooks/{webhook_id}`

**Request:**
```json
{
  "active": false
}
```

**Response:**
```json
{
  "id": "hook_123",
  "active": false
}
```

### Delete Webhook

**Endpoint:** `DELETE /api/webhooks/{webhook_id}`

**Response:**
```json
{
  "status": "deleted"
}
```

### Test Webhook

**Endpoint:** `POST /api/webhooks/{webhook_id}/test`

Sends a test event to the webhook endpoint.

**Response:**
```json
{
  "status": "test_sent"
}
```

### Get Webhook Logs

**Endpoint:** `GET /api/webhooks/{webhook_id}/logs?limit=50`

Returns delivery logs for a specific webhook.

**Query Parameters:**
- `limit` (optional, default: 50) - Number of logs to return

**Response:**
```json
{
  "logs": [
    {
      "id": "evt_456",
      "event": "search.created",
      "correlation_id": "search_123",
      "status": "success",
      "response_status": 200,
      "error_message": null,
      "timestamp": 1714871200
    }
  ]
}
```

## Webhook Payload Format

When a webhook is triggered, your server will receive an HTTP POST request with the following structure:

### Headers
```
X-Webhook-Signature: t=1714871200, hex_signature_here
X-Webhook-Event: webhook
Content-Type: application/json
```

### Body
```json
{
  "id": "evt_123",
  "event": "search.created",
  "timestamp": 1714871200,
  "correlation_id": "search_42",
  "data": {
    "token": 42,
    "query": "music artist",
    "target": "global",
    "target_name": null,
    "result_count": 156
  }
}
```

## Event-Specific Payload Examples

### search.created
```json
{
  "token": 42,
  "query": "music artist",
  "target": "global",
  "target_name": null,
  "result_count": 156
}
```

### search.completed
```json
{
  "token": 42,
  "query": "music artist",
  "result_count": 156,
  "target": "global"
}
```

### transfer.completed
```json
{
  "transfer_id": 1,
  "filename": "song.mp3",
  "peer_username": "peer_username",
  "direction": "download",
  "size": 5242880,
  "bytes_transferred": 5242880,
  "status": "succeeded"
}
```

### message.sent
```json
{
  "message_id": 999,
  "username": "peer_username",
  "body": "Hello, friend!",
  "direction": "outbound"
}
```

## Security

### Signature Verification

Each webhook request includes an HMAC-SHA256 signature in the `X-Webhook-Signature` header. You should verify this signature to ensure the request is authentic.

**Example verification (Python):**
```python
import hmac
import hashlib

def verify_webhook_signature(request_body, signature_header, webhook_secret):
    parts = signature_header.split(", ")
    timestamp = parts[0].split("=")[1]
    signature = parts[1]
    
    msg = request_body.encode()
    expected = hmac.new(
        webhook_secret.encode(),
        msg,
        hashlib.sha256
    ).hexdigest()
    
    return hmac.compare_digest(signature, expected)
```

**Example verification (JavaScript/Node.js):**
```javascript
const crypto = require('crypto');

function verifyWebhookSignature(requestBody, signatureHeader, webhookSecret) {
  const parts = signatureHeader.split(", ");
  const timestamp = parts[0].split("=")[1];
  const signature = parts[1];
  
  const expected = crypto
    .createHmac('sha256', webhookSecret)
    .update(requestBody)
    .digest('hex');
  
  return crypto.timingSafeEqual(signature, expected);
}
```

## Retry Policy

Failed webhook deliveries will be retried up to `max_retries` times (default: 3). Each webhook can have:
- `retry_count`: Number of times delivery has been attempted
- `max_retries`: Maximum number of retries (default: 3)
- `timeout_seconds`: HTTP request timeout (default: 30 seconds)

## Best Practices

1. **Verify Signatures**: Always verify the HMAC signature to ensure requests are authentic
2. **Handle Idempotently**: Use the `id` field in the webhook payload as an idempotency key
3. **Respond Quickly**: Return a 2xx status code as quickly as possible
4. **Use Correlation IDs**: The `correlation_id` field links related events
5. **Monitor Logs**: Regularly check `/api/webhooks/{id}/logs` to monitor delivery
6. **Use HTTPS**: Always use HTTPS for your webhook endpoint URL
7. **Rotate Secrets**: Periodically regenerate webhook secrets for security

## Example Webhook Handler (Python Flask)

```python
from flask import Flask, request, jsonify
import hmac
import hashlib

app = Flask(__name__)
WEBHOOK_SECRET = "your-webhook-secret"

@app.route('/webhook', methods=['POST'])
def webhook():
    # Verify signature
    signature_header = request.headers.get('X-Webhook-Signature')
    body = request.get_data()
    
    parts = signature_header.split(", ")
    signature = parts[1]
    
    expected = hmac.new(
        WEBHOOK_SECRET.encode(),
        body,
        hashlib.sha256
    ).hexdigest()
    
    if not hmac.compare_digest(signature, expected):
        return jsonify({"error": "Invalid signature"}), 401
    
    # Process the webhook
    data = request.json
    event_type = data['event']
    correlation_id = data['correlation_id']
    event_data = data['data']
    
    if event_type == 'search.created':
        print(f"Search created: {event_data['query']}")
    elif event_type == 'transfer.completed':
        print(f"Transfer completed: {event_data['filename']}")
    
    return jsonify({"status": "received"}), 200

if __name__ == '__main__':
    app.run(port=5000, ssl_context='adhoc')
```

## Example Webhook Handler (JavaScript Express)

```javascript
const express = require('express');
const crypto = require('crypto');
const app = express();

app.use(express.json());

const WEBHOOK_SECRET = 'your-webhook-secret';

app.post('/webhook', (req, res) => {
  // Verify signature
  const signatureHeader = req.headers['x-webhook-signature'];
  const body = JSON.stringify(req.body);
  
  const parts = signatureHeader.split(", ");
  const signature = parts[1];
  
  const expected = crypto
    .createHmac('sha256', WEBHOOK_SECRET)
    .update(body)
    .digest('hex');
  
  if (!crypto.timingSafeEqual(signature, expected)) {
    return res.status(401).json({ error: 'Invalid signature' });
  }
  
  // Process the webhook
  const { event, correlation_id, data } = req.body;
  
  if (event === 'search.created') {
    console.log(`Search created: ${data.query}`);
  } else if (event === 'transfer.completed') {
    console.log(`Transfer completed: ${data.filename}`);
  }
  
  res.json({ status: 'received' });
});

app.listen(5000);
```

## Troubleshooting

### Webhook not being triggered
1. Check if the webhook is active: `PATCH /api/webhooks/{id}` with `{"active": true}`
2. Verify the events are configured correctly
3. Check the webhook logs: `GET /api/webhooks/{id}/logs`
4. Ensure your endpoint is accessible from the server

### "Failed to deliver" in logs
1. Check HTTP response status code from your endpoint
2. Verify endpoint is returning 2xx status
3. Check if endpoint is timing out (default 30 seconds)
4. Review server logs for errors

### Signature verification failing
1. Ensure you're using the correct webhook secret
2. Verify you're signing the raw request body, not parsed JSON
3. Use constant-time comparison (`timingSafeEqual` in Node, `compare_digest` in Python)

## Rate Limiting

Webhook delivery requests are not rate-limited, but your server should handle the incoming load appropriately. The default behavior is to dispatch webhooks asynchronously without blocking the main request handling.

## Compliance & Standards

- Webhook signatures use HMAC-SHA256 (industry standard)
- Payloads are sent as JSON with UTF-8 encoding
- Timestamp format is Unix epoch (seconds since Jan 1, 1970)
- All endpoints support standard HTTP status codes
