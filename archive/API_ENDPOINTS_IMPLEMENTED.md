# slskr API Endpoints - Complete Implementation Status

**Last Updated:** 2026-05-04 (Phase 8 Implementation)

## Summary

- **Total Endpoints Implemented:** 85+
- **API Versions:** v0, legacy (/api)
- **Test Coverage:** 151 passing unit tests
- **Database Backing:** SQLite with async support

---

## Core System Endpoints

### Health & Status
| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/health` | GET | ✅ | Basic health check |
| `/api/version` | GET | ✅ | Version information |
| `/api/v0/health/detailed` | GET | ✅ | Expanded health with counters |
| `/api/v0/diagnostics` | GET | ✅ | System diagnostics and metrics |

### Configuration
| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/config` | GET | ✅ | Current configuration (sanitized) |
| `/api/stats` | GET | ✅ | Global statistics |
| `/api/telemetry` | GET | ✅ | Runtime health metrics |
| `/api/metrics` | GET | ✅ | Performance metrics |

### Database Management
| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/v0/database/stats` | GET | ✅ | Database statistics (searches, transfers, messages, users, rooms) |
| `/api/v0/database/cleanup` | POST | ✅ | Delete old records (older than N days) |
| `/api/v0/database/vacuum` | POST | ✅ | Optimize database storage |

---

## Session Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/session` | GET | ✅ | Current session status |
| `/api/session/connect` | POST | ✅ | Connect to server |
| `/api/session/disconnect` | POST | ✅ | Disconnect from server |
| `/api/session/ping` | POST | ✅ | Send keepalive ping |
| `/api/session/privileges/check` | POST | ✅ | Check user privileges |

---

## Search Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/searches` | GET | ✅ | List searches with filters |
| `/api/searches` | POST | ✅ | Create new search |
| `/api/searches/{id}` | GET | ✅ | Get search details |
| `/api/search-responses` | POST | ✅ | Ingest search responses |
| `/api/searches/{id}/responses` | GET | ✅ | Get search responses |

---

## Transfer Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/transfers` | GET | ✅ | List transfers with filters |
| `/api/transfers` | POST | ✅ | Create transfer |
| `/api/transfers/{id}` | GET | ✅ | Get transfer details |
| `/api/transfers/{id}` | DELETE | ✅ | Cancel transfer |
| `/api/transfers/{id}/start` | POST | ✅ | Start transfer |
| `/api/transfers/{id}/progress` | POST | ✅ | Update progress |
| `/api/transfers/{id}/complete` | POST | ✅ | Mark as complete |
| `/api/transfers/stats` | GET | ✅ | Transfer statistics |
| `/api/v0/transfers` | GET/POST | ✅ | V0 API transfers |

---

## Message Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/messages` | GET | ✅ | List all messages |
| `/api/messages` | POST | ✅ | Send message |
| `/api/messages/{id}/ack` | POST | ✅ | Acknowledge message (POST) |
| `/api/messages/{id}/acknowledge` | PUT | ✅ | Acknowledge message (PUT) |
| `/api/messages/{username}` | GET | ✅ | Get messages from user |

---

## Room Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/rooms` | GET | ✅ | List rooms with filters |
| `/api/rooms/refresh` | POST | ✅ | Refresh room list from server |
| `/api/rooms/{name}/join` | POST | ✅ | Join room |
| `/api/rooms/{name}/leave` | DELETE | ✅ | Leave room |
| `/api/rooms/{name}/messages` | POST | ✅ | Post room message |
| `/api/rooms/{name}/messages` | GET | ✅ | Get room messages |
| `/api/rooms/{name}/users` | GET | ✅ | Get room users |
| `/api/v0/rooms` | GET | ✅ | V0 API rooms |
| `/api/rooms/available` | GET | ✅ | Available rooms |

---

## User Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/users` | GET | ✅ | List watched users |
| `/api/users/watch` | POST | ✅ | Watch user |
| `/api/users/{username}/watch` | DELETE | ✅ | Unwatch user |
| `/api/users/{username}/stats` | GET | ✅ | Get user statistics |
| `/api/users/{username}/stats/request` | POST | ✅ | Request user stats |
| `/api/users/{username}/browse` | GET | ✅ | Get user browse session |
| `/api/users/{username}/browse/request` | POST | ✅ | Request user browse |
| `/api/users/{username}/browse/folder` | POST | ✅ | Browse user folder |
| `/api/v0/users/watch` | POST | ✅ | V0 API watch user |

---

## Browse Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/browse` | GET | ✅ | List browse sessions |
| `/api/browse-responses` | POST | ✅ | Ingest browse responses |

---

## Share Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/shares` | GET | ✅ | List shares |
| `/api/shares/catalog` | GET | ✅ | Full share catalog |
| `/api/files/{path}` | GET | ✅ | Get shared file |
| `/api/shares/rescan` | POST | ✅ | Rescan shares |

---

## Listener Endpoints

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/listeners` | GET | ✅ | List listeners |

---

## WebSocket & Events

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/events` | GET | ✅ | Get event stream (SSE) |
| `/ws` | WebSocket | ✅ | WebSocket connection |

---

## Webhook Endpoints (Stubs)

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/webhooks` | GET | ⏳ | List webhooks |
| `/api/webhooks` | POST | ⏳ | Create webhook |
| `/api/webhooks/{id}` | GET | ⏳ | Get webhook |
| `/api/webhooks/{id}` | DELETE | ⏳ | Delete webhook |
| `/api/webhooks/{id}/test` | POST | ⏳ | Test webhook |

**Status:** Endpoints exist but event dispatch not wired

---

## API Key Management (Stubs)

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/v0/api-keys` | GET | ⏳ | List API keys |
| `/api/v0/api-keys` | POST | ⏳ | Create API key |
| `/api/v0/api-keys/{id}` | DELETE | ⏳ | Revoke API key |

**Status:** Endpoints exist but no actual key generation/validation

---

## GraphQL Endpoints (Schema Only)

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/graphql/schema` | GET | ⏳ | GraphQL schema |
| `/api/graphql` | POST | ⏳ | GraphQL query execution |

**Status:** Schema defined but query engine not wired

---

## v0 API Versioning

The following endpoints support v0 API versioning:

- `/api/v0/database/*` - Database maintenance
- `/api/v0/diagnostics` - System diagnostics
- `/api/v0/health/detailed` - Detailed health
- `/api/v0/transfers` - Transfer operations
- `/api/v0/messages` - Message operations
- `/api/v0/searches` - Search operations
- `/api/v0/rooms` - Room operations
- `/api/v0/users/watch` - User watch operations
- `/api/v0/api-keys` - API key management
- `/api/v0/browse-responses` - Browse responses

---

## Authentication & Authorization

**Supported Methods:**
- Bearer token authentication
- API key (HMAC-SHA256 signed requests)
- Cookie-based sessions
- CSRF protection (origin checks)

**Protected Routes:**
- All POST/PUT/DELETE operations (unless disabled)
- Admin endpoints (require authentication)
- API key endpoints (require authentication)

---

## Content Types

- **Request:** `application/json` (JSON body)
- **Response:** `application/json` (all endpoints)
- **Error Format:** `{"error": "message"}`

---

## HTTP Status Codes

| Code | Usage |
|------|-------|
| 200 | Successful GET/PUT request |
| 201 | Successful resource creation |
| 202 | Accepted (async operation) |
| 400 | Bad request (validation error) |
| 401 | Unauthorized (auth required) |
| 403 | Forbidden (CSRF or permission) |
| 404 | Not found |
| 409 | Conflict (state violation) |
| 501 | Not implemented (stub endpoint) |

---

## Recent Additions (This Session)

### Database Maintenance Endpoints
- `GET /api/v0/database/stats` - Get database statistics
- `POST /api/v0/database/cleanup` - Clean old records
- `POST /api/v0/database/vacuum` - Optimize storage

### Health & Diagnostics
- `GET /api/v0/health/detailed` - Expanded health metrics
- `GET /api/v0/diagnostics` - System diagnostics

### Transfer Operations
- `GET /api/transfers/{id}` - Get transfer details
- `DELETE /api/transfers/{id}` - Cancel transfer

### Message Operations
- `PUT /api/messages/{id}/acknowledge` - Mark read (PUT method)

---

## Known Limitations & Gaps

### Not Fully Implemented
1. **Webhook Event Dispatch** - Event types defined but not triggered from application
2. **GraphQL Query Engine** - Schema defined but query routing not implemented
3. **API Key Storage** - Creation endpoints exist but keys not persisted
4. **Database Persistence** - Tables created but not actively written to on most operations
5. **Real-time Updates** - WebSocket framework present but limited event streaming

### Architectural Notes
- API uses custom JSON parsing (no serde for routing)
- No ORM; direct SQL queries for database operations
- Request bodies parsed with manual string scanning for performance
- All endpoints are async with tokio runtime
- No request middleware pipeline (validation/auth done per endpoint)

---

## API Design Patterns

### List Endpoints
Query parameters for filtering and pagination:
- `q=query` - Text filter
- `status=value` - Status filter
- `limit=10` - Result limit (default: 50)
- `offset=0` - Result offset (default: 0)
- `joined=true` - Boolean filter (rooms)

### Resource Creation
- POST body contains resource fields
- Returns 201 Created with full resource
- Returns location header (not implemented)

### Resource Updates
- PUT for full replacement (not common)
- POST for actions/state transitions
- PATCH not used (PUT used instead)

### Resource Deletion
- DELETE removes resource
- Returns 200 OK or 404 Not Found

### Error Responses
All errors return JSON with `error` field:
```json
{"error": "descriptive message"}
```

---

## Testing

### Unit Tests
- 151 passing tests covering:
  - Protocol message handling
  - API endpoint behavior
  - State management
  - Edge cases

### Integration Testing
See `scripts/` directory:
- `run-live-soak-24h.sh` - 24-hour stability test
- `run-proton-public-matrix.sh` - Real network testing
- `run-live-soak-proton-natpmp.sh` - NAT-PMP validation

---

## Performance Notes

### Optimizations Implemented
- Response caching for expensive operations
- Database indices on frequently queried columns
- Async/await throughout (no blocking)
- Connection pooling for database

### Metrics Available
- Request timing (correlation IDs, millisecond precision)
- Transfer statistics (bytes, counts, rates)
- Search response aggregation
- Memory usage (via `/api/metrics`)

---

## Future Enhancements

1. **OpenAPI/Swagger UI** - Auto-generated documentation
2. **gRPC API** - Alternative transport layer
3. **Server-Sent Events** - Better real-time streaming
4. **Rate Limiting** - Per-endpoint or per-user
5. **Request Validation** - JSON schema validation
6. **Response Pagination** - Standardized cursors
7. **Batch Operations** - Bulk create/update/delete
8. **Change Streams** - Watch for changes (MongoDB-like)

---

## Related Documentation

- `docs/http-api.md` - Detailed HTTP API guide
- `docs/http-api-features.md` - Feature coverage
- `docs/app-surface.md` - App surface diagram
- `REMAINING_IMPLEMENTATION_PLAN.md` - Next steps
- `docs/openapi.json` - OpenAPI 3.0 schema (partial)

