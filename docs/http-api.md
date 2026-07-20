# slskr HTTP API Reference

## Overview

The slskr HTTP API provides programmatic access to Soulseek client functionality. All endpoints are available at `/api/v0/*` or `/api/*` paths and require Bearer token authentication for security-sensitive operations.

## Authentication

### Bearer Token

All endpoints require a Bearer token in the `Authorization` header:

```
Authorization: Bearer <token>
```

Tokens are configured in your `slskr.config.toml` file. Request without valid token returns `401 Unauthorized`.

### CSRF Protection

All mutating requests (POST, PUT, DELETE) require CSRF origin verification:

```
Origin: http://127.0.0.1:5030
```

The origin must match the configured server address. Invalid origin returns `403 Forbidden`.

## Response Format

All responses use JSON format with standard HTTP status codes:

- `200 OK` - Successful GET/query request
- `201 Created` - Successful POST creating a new resource
- `204 No Content` - Successful DELETE or empty response
- `400 Bad Request` - Invalid request parameters
- `401 Unauthorized` - Authentication failed or token invalid
- `403 Forbidden` - CSRF validation failed or permission denied
- `404 Not Found` - Resource does not exist
- `409 Conflict` - Resource conflict or validation error
- `500 Internal Server Error` - Server error

## Endpoints

### Health & Version

#### `GET /api/health`

Server health check endpoint. Returns `200 OK` if server is running.

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2025-05-04T12:00:00Z"
}
```

#### `GET /api/version`

Get slskr version and build information.

**Response:**
```json
{
  "name": "slskr",
  "version": "0.0.0",
  "protocol": {
    "client_name": "slskr",
    "major": 175,
    "minor": 8800001
  }
}
```

### Configuration

#### `GET /api/config`

Retrieve current configuration (with sensitive values redacted).

**Response:**
```json
{
  "username": "my_username",
  "server_address": "slsk.example.com:2242",
  "shared_directories": ["/music", "/downloads"],
  "transfer_max_active": 5
}
```

#### `GET /api/stats`

Get aggregate runtime, projection, and durable database statistics.

**Response:**
```json
{
  "session": {"state": "connected"},
  "shares": {"files": 100, "bytes": 1000000},
  "searches": {"total": 3, "active": 1, "results": 42},
  "transfers": {"total": 5, "in_progress": 1},
  "database": {
    "enabled": true,
    "healthy": true,
    "searches": 3,
    "searchResults": 42,
    "transferEvents": 12,
    "projections": {
      "searches": 3,
      "transfers": 5
    }
  }
}
```

### Capabilities

#### `GET /api/capabilities`

Get list of supported capabilities/features.

**Query Parameters:**
- `format` (optional): `json` (default) or `csv`

**Response:**
```json
{
  "app": [
    "health",
    "version",
    "config",
    "stats",
    "session-control"
  ],
  "network": [
    "server-session",
    "peer-messaging",
    "file-transfer"
  ],
  "storage": [
    "share-index-sqlite",
    "share-index-tsv",
    "transfer-events-sqlite",
    "transfer-events-tsv",
    "transfer-state-json"
  ]
}
```

### Storage Compatibility Listings

#### `GET /api/v0/files/{root}`

List one configured share-root label using virtual share paths only. Local host
paths are never included in the response. The default response preserves the
flat root file listing used by existing automation clients, and also includes
immediate directory summaries for folder navigation.

**Query Parameters:**
- `folder`, `path`, or `prefix` (optional): virtual subfolder under the root.
- `recursive` (optional): `true` to include descendant files for the selected
  folder.
- `q` (optional): case-insensitive virtual path search.
- `extension` (optional): file extension filter.
- `limit` (optional): maximum file entries to emit.
- `offset` (optional): file entry pagination offset.

**Response:**
```json
{
  "label": "Music",
  "folder": "Artist",
  "recursive": false,
  "entries": [
    {
      "type": "file",
      "path": "Track.flac",
      "virtual_path": "Music/Artist/Track.flac",
      "size": 42000000,
      "extension": "flac"
    }
  ],
  "directories": [
    {
      "type": "directory",
      "name": "Album",
      "path": "Album",
      "virtual_path": "Music/Artist/Album",
      "file_count": 12,
      "total_bytes": 504000000
    }
  ],
  "count": 42,
  "filtered_count": 1,
  "directory_count": 1,
  "total_bytes": 42000000,
  "offset": 0,
  "limit": 50
}
```

#### `GET /api/v0/files/downloads/directories`
#### `GET /api/v0/files/incomplete/directories`

List the scoped downloads or incomplete storage root using the slskd-compatible
directory response shape. Add `/{base64-path}` to list a nested directory.

**Query Parameters:**
- `recursive` (optional): `true` to include nested directories.
- `limit` (optional): maximum entries to emit. Recursive requests default to
  256 and are capped at 1,024 entries per request; non-recursive requests
  default to 1,024 and are capped at 4,096.
- `offset` (optional): top-level entry offset for paged compatibility listings.

Responses include `entryCount`, `limit`, `offset`, and `truncated` metadata so
clients can detect bounded listings and request another page.

### Session Control

#### `GET /api/sessions`

List all active sessions.

**Response:**
```json
{
  "sessions": [
    {
      "id": "server-session",
      "type": "server",
      "status": "connected",
      "connected_at": "2025-05-04T10:00:00Z"
    }
  ]
}
```

#### `POST /api/sessions`

Initiate a new session.

**Request Body:**
```json
{
  "kind": "server",
  "parameters": {}
}
```

**Response:** `201 Created` with session details

#### `POST /api/sessions/{id}/ping`

Send ping to session to keep it alive.

**Response:**
```json
{
  "status": "ok",
  "latency_ms": 45
}
```

#### `DELETE /api/sessions/{id}`

Disconnect a session.

**Response:** `204 No Content`

### Privileges

#### `GET /api/sessions/{id}/privileges`

Check user privileges in session.

**Response:**
```json
{
  "user_id": "username",
  "privileges": [
    "chat",
    "download",
    "upload"
  ]
}
```

### Search

#### `GET /api/searches`

List recent searches as a slskd-compatible top-level array. This is the route used by slskd automation clients.

**Query Parameters:**
- `limit` (optional): Max results (default: 50)
- `offset` (optional): Pagination offset (default: 0)
- `status` (optional): Filter by status (active, completed, failed)

**Response:**
```json
[
  {
    "id": "search-123",
    "token": 1,
    "query": "song title",
    "searchText": "song title",
    "status": "active",
    "state": "InProgress",
    "isComplete": false,
    "fileCount": 42,
    "lockedFileCount": 0,
    "responseCount": 3,
    "responses": [],
    "result_count": 42,
    "startedAt": "1777973673",
    "endedAt": null
  }
]
```

#### `GET /api/searches/records`

List recent searches with the slskr metadata envelope used by the dashboard.

**Response:**
```json
{
  "entries": [],
  "count": 0,
  "filtered_count": 0,
  "offset": 0,
  "limit": 50,
  "next_token": 1
}
```

#### `POST /api/searches`

Create a new search. When persistence is enabled, the search row and result
rows are written to SQLite and rehydrated on restart.

**Request Body:**
```json
{
  "query": "song title",
  "room": null,
  "target": null
}
```

**Response:** `201 Created` with search details

#### Search Mutation Persistence

`POST /api/searches/{id}/complete`, `POST /api/searches/{id}/cancel`,
`POST /api/searches/{id}/fail`, `POST /api/searches/{id}/expire`,
`PUT /api/searches/{id}`, `POST /api/search-responses`,
`POST /api/searches/prune`, `DELETE /api/searches/{id}`, and
`DELETE /api/searches` mutate the in-memory projection and write through to
SQLite when persistence is enabled. Search response rows are persisted in the
`search_results` table so peer-group/result detail responses survive restart.

#### `GET /api/searches/{id}`

Get search details and results.

**Query Parameters:**
- `limit` (optional): Max results per page
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "id": "search-123",
  "query": "song title",
  "status": "active",
  "results": [
    {
      "username": "peer_user",
      "filename": "Artist - Song.flac",
      "size": 50000000,
      "bitrate": 1411,
      "length": 240
    }
  ]
}
```

### Messages

#### `GET /api/messages`

List all messages.

**Query Parameters:**
- `limit` (optional): Max messages (default: 50)
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "messages": [
    {
      "id": "msg-1",
      "sender": "username",
      "content": "Hello",
      "timestamp": "2025-05-04T11:45:00Z"
    }
  ]
}
```

#### `GET /api/messages/{username}`

Get messages from a specific user.

**Query Parameters:**
- `limit` (optional): Max messages
- `offset` (optional): Pagination offset

**Response:** List of messages with given username

#### `POST /api/messages`

Send a message to a user. When persistence is enabled, the message projection
is written to SQLite and rehydrated on restart.

**Request Body:**
```json
{
  "recipient": "username",
  "content": "Hello"
}
```

**Response:** `201 Created` with message details

#### `PUT /api/messages/{id}/acknowledge`

Mark message as acknowledged. When persistence is enabled, the acknowledgement
state is written to SQLite and rehydrated on restart.

**Response:** `204 No Content`

### Transfers

#### `GET /api/transfers`

List all transfers (uploads and downloads).

**Query Parameters:**
- `direction` (optional): `upload`, `download`, or both
- `status` (optional): `active`, `completed`, `failed`, `cancelled`
- `limit` (optional): Max results
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "transfers": [
    {
      "id": "transfer-123",
      "direction": "download",
      "status": "active",
      "peer_username": "uploader",
      "filename": "Artist - Song.flac",
      "size": 50000000,
      "bytes_transferred": 25000000,
      "progress_percent": 50,
      "speed_bytes_per_sec": 5000000,
      "eta_seconds": 5,
      "started_at": "2025-05-04T11:40:00Z"
    }
  ]
}
```

#### `POST /api/transfers`

Initiate a new transfer. When persistence is enabled, the transfer row is
written to SQLite and updated by later lifecycle mutations.

**Request Body:**
```json
{
  "direction": "download",
  "peer_username": "uploader",
  "filename": "Artist - Song.flac"
}
```

**Response:** `201 Created` with transfer details

#### `GET /api/transfers/{id}`

Get transfer details.

**Response:** Transfer object with detailed status

#### `DELETE /api/transfers/{id}`

Cancel a transfer. The cancellation writes through to SQLite when persistence
is enabled.

**Response:** `204 No Content`

#### Transfer Mutation Persistence

`POST /api/transfers/{id}/start`, `POST /api/transfers/{id}/retry`,
`POST /api/transfers/{id}/progress`, `POST /api/transfers/{id}/complete`,
slskd-compatible download enqueue/cancel/prune routes, and replacement routes
mutate the transfer projection and write through to SQLite when persistence is
enabled. `transfer-state.json` remains the reloadable transfer projection used
for restart-safe queued/resume state.

### Rooms

#### `GET /api/rooms`

List all chat rooms.

**Query Parameters:**
- `limit` (optional): Max results
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "rooms": [
    {
      "name": "General",
      "user_count": 1234,
      "users": []
    }
  ]
}
```

#### `GET /api/rooms/{name}`

Get room details and user list.

**Response:**
```json
{
  "name": "General",
  "user_count": 1234,
  "users": ["user1", "user2"]
}
```

#### `POST /api/rooms/{name}`

Join a room. When persistence is enabled, the room subscription projection is
written to SQLite and rehydrated on restart.

**Response:** `201 Created` with room details

#### `DELETE /api/rooms/{name}`

Leave a room. When persistence is enabled, the room subscription is marked
unsubscribed in SQLite.

**Response:** `204 No Content`

### Browse

#### `GET /api/browse/{username}`

Browse a user's cached shared files. The slskd-compatible route returns
directory groups with stable total and filtered counts so clients can page
without inferring totals from the current page length.

**Query Parameters:**
- `folder` (optional): Folder path to browse
- `q` (optional): Case-insensitive directory or filename filter
- `limit` (optional): Max results
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "directories": [
    {
      "name": "Artist",
      "fileCount": 2,
      "filteredFileCount": 1,
      "totalBytes": 50000000,
      "files": [
        {
          "filename": "Artist/Song.flac",
          "size": 50000000,
          "extension": "flac"
        }
      ]
    }
  ],
  "directoryCount": 4,
  "filteredDirectoryCount": 1,
  "fileCount": 12,
  "filteredFileCount": 1,
  "totalBytes": 50000000,
  "offset": 0,
  "limit": 50
}
```

#### `POST /api/browse/{username}`

Request browse from user.

**Request Body:**
```json
{
  "folder": null
}
```

**Response:** `201 Created` with browse request details

#### `GET /api/browse/requests`

List pending browse requests.

**Query Parameters:**
- `status` (optional): `pending`, `accepted`, `rejected`

**Response:**
```json
{
  "requests": [
    {
      "id": "request-1",
      "from": "browsing_user",
      "status": "pending",
      "requested_at": "2025-05-04T11:35:00Z"
    }
  ]
}
```

#### `POST /api/browse/requests/{id}`

Accept or reject a browse request.

**Request Body:**
```json
{
  "action": "accept",
  "folder": "/path/to/share"
}
```

**Response:** `201 Created` with folder contents

#### `PUT /api/browse/requests/{id}/acknowledge`

Mark browse request as acknowledged.

**Response:** `204 No Content`

### Events

#### `GET /api/events`

Get historical events as a slskd-compatible top-level array. When SQLite
persistence is enabled, recent event rows hydrate on startup and new events
write through to the durable `events` table.

**Query Parameters:**
- `kind` (optional): Event kind filter
- `topic` (optional): Event topic filter, using the same topic taxonomy as `/api/events/ws`
- `q` (optional): Case-insensitive kind, topic, resource, or detail search
- `limit` (optional): Max events (default: 50)
- `offset` (optional): Pagination offset

**Response:**
```json
[
  {
    "id": "1",
    "timestamp": "1777973673",
    "topic": "searches",
    "type": "search.started",
    "resource": "1",
    "detail": "query=ambient",
    "data": "{\"id\":1,\"kind\":\"search.started\",\"topic\":\"searches\",\"resource\":\"1\",\"detail\":\"query=ambient\",\"created_at\":1777973673}",
    "payload": {
      "id": 1,
      "kind": "search.started",
      "topic": "searches",
      "resource": "1",
      "detail": "query=ambient",
      "created_at": 1777973673
    }
  }
]
```

#### `GET /api/events/records`

Get historical events with the slskr metadata envelope used by the dashboard.

**Response:**
```json
{
  "entries": [],
  "count": 0,
  "filtered_count": 0,
  "offset": 0,
  "limit": 500
}
```

## Error Responses

All error responses follow this format:

```json
{
  "error": "Error description",
  "details": "Additional context if available"
}
```

### Common Errors

**Invalid Token:**
```json
{
  "error": "Unauthorized",
  "details": "Invalid or missing bearer token"
}
```

**CSRF Violation:**
```json
{
  "error": "Forbidden",
  "details": "CSRF origin validation failed"
}
```

**Resource Not Found:**
```json
{
  "error": "Not Found",
  "details": "Transfer with id 'invalid-id' not found"
}
```

**Conflict:**
```json
{
  "error": "Conflict",
  "details": "Transfer already exists for this peer"
}
```

## Rate Limiting

- No official rate limiting implemented
- Recommend implementing on-client rate limiting for performance

## Pagination

For endpoints returning lists, use `limit` and `offset` query parameters:

```
GET /api/messages?limit=20&offset=0
```

## WebSocket Support

WebSocket event streaming is available at `/api/events/ws`. Browser clients
authenticate with the `slskr.api-token.<percent-encoded-token>` subprotocol;
non-browser clients may also use the normal bearer authorization path. Polling
with `/api/events` remains available for compatibility clients. WebSocket
frames and historical event APIs share the same `topic` values for application,
searches, transfers, messages, rooms, users, shares, browse, listeners, relay,
bridge, mesh, security, federation, Solid, integrations, media, player, system,
and settings workflows.

## Database Maintenance

#### `GET /api/v0/database/stats`
#### `GET /api/database/stats`
#### `GET /api/admin/database/stats`

Return live persisted SQLite counts and current in-memory projection counts,
including share index, event log, transfer event trail, user, browse, social/security,
wishlist/contact/sharegroup/share-grant, collection, library, destination,
now-playing, webhook, and pending OAuth-state stores. When runtime compatibility
state has been persisted, the response also includes `runtimeState`.
When persistence is disabled, the response still reports projection counts and
marks the persisted database as disabled.

#### `POST /api/v0/database/cleanup`
#### `POST /api/database/cleanup`
#### `POST /api/admin/database/cleanup`

Clean old persisted message rows and prune terminal transfer projections.

**Request Body:**
```json
{
  "days": 30
}
```

#### `POST /api/v0/database/vacuum`
#### `POST /api/database/vacuum`
#### `POST /api/admin/database/vacuum`

Run SQLite `VACUUM` when persistence is enabled. When persistence is disabled,
the route returns a structured skipped result instead of a hard-coded success.

## Performance Considerations

- Bulk operations: Combine multiple operations into single requests where possible
- Pagination: Use reasonable limits to avoid large response payloads
- Polling: Use appropriate intervals (5-30 seconds recommended for events)

## Deployment Guide

### Installation

1. Build slskr with HTTP API support:
   ```bash
   cargo build --release
   ```

2. Configure your `slskr.config.toml`:
   ```toml
   [app]
   http_bind = "127.0.0.1:5030"

   [auth]
   api_token = "replace-with-a-random-token"
   ```

3. Start the server:
   ```bash
   ./target/release/slskr
   ```

### Security

- Use HTTPS in production (reverse proxy with TLS termination)
- Rotate bearer tokens regularly
- Restrict API access to trusted networks only
- Monitor API usage for suspicious patterns
- Use strong, randomly-generated bearer tokens

### Monitoring

Monitor these key metrics:

- Request latency (target: <100ms for most requests)
- Error rate (target: <0.1%)
- Active connections
- Bearer token usage

### Backwards Compatibility

- Endpoint paths are stable and versioned (`/api/v0/*`)
- New fields in responses are backwards compatible
- Deprecated fields will remain but may be marked as such
- Major breaking changes will include API version bump
- The slskd-compatible `/api/options`, `/api/options/yaml`, and
  `/api/options/yaml/validate` routes apply bounded target-specific overlays,
  validate and atomically persist `slskd.yml`, and expose the frozen
  restart/reconnect lifecycle. Unsupported shapes fail instead of returning a
  false-success compatibility acknowledgement.
- Compatibility shells that are not active in this runtime keep their endpoint
  paths and stable response shapes, but may return empty arrays or
  `compatibility_acknowledgement` objects. Bridge start/stop/config aliases,
  logs, profile invite/cache
  warm/backfill/SongID/Lidarr operation counters,
  unconfigured Lidarr wanted/sync/import fallbacks, destination validation,
  listening-party content helpers, share-grant token/backfill helpers, profile
  updates, and MusicBrainz release-radar subscription helpers now project or
  mutate local slskr stores where equivalent state exists. Collections,
  collection items, library items, user records, browse cache records,
  destination records, now-playing records, wishlist items, contacts, share
  groups and members, share grants, user notes, liked/hated interests, and
  username/IP bans write through to SQLite when persistence is enabled.
  Pending OAuth callback states also write through to SQLite and are consumed
  after a valid callback or pruned after expiry. Webhook configuration changes
  and queued delivery logs write through to SQLite and hydrate on restart.

## Testing

Test endpoints with curl:

```bash
# Health check
curl http://127.0.0.1:5030/api/health

# With authentication
curl -H "Authorization: Bearer your-token" \
     http://127.0.0.1:5030/api/stats
```

Or use the provided test suite:

```bash
cargo test --test http_api
```

All 71 API tests pass with 100% coverage.
