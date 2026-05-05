# Webhook API

Webhook endpoints are protected API routes. Requests must use the same bearer or `X-API-Key` authentication as the rest of the protected HTTP API.

## Secret Lifecycle

`POST /api/webhooks` and `POST /api/admin/webhooks` return the webhook signing secret only in the creation response. Treat this as a one-time display value. List, detail, delete, patch, test, and log routes do not return webhook secrets.

If the creation response is lost, delete and recreate the webhook with a new generated secret or provide a new explicit `secret` field at creation time.

Webhook deliveries sign the JSON payload with `X-Webhook-Signature` using HMAC-SHA256. Delivery validation rejects localhost, private, loopback, link-local, and otherwise blocked webhook targets at registration where possible and again at delivery after DNS resolution.

## Create

```http
POST /api/webhooks
Authorization: Bearer <api-token>
Content-Type: application/json

{"url":"https://example.com/slskr/webhook","events":"search.created,transfer.completed"}
```

Response:

```json
{
  "id": "hook_0",
  "secret": "secret_generated_value",
  "secretReturnedOnce": true,
  "status": "created"
}
```

## List

```http
GET /api/webhooks
Authorization: Bearer <api-token>
```

List responses include id, URL, events, active state, retry settings, and timestamps. They intentionally omit `secret`.
