# API Versioning Strategy

## Overview

slskR implements semantic versioning for the HTTP API to support evolution while maintaining backward compatibility. Three API versions are supported:

- **v0 (Legacy):** `/api/*` - Original endpoints (kept for compatibility)
- **v1 (Current):** `/api/v1/*` - Current stable API
- **v2 (Future):** `/api/v2/*` - Enhanced endpoints with new features

## URL Structure

All three versions are equivalent for now, but v1 is the recommended version for new clients:

```
GET /api/health                    # v0/legacy
GET /api/v1/health                 # v1 (recommended)
GET /api/v2/health                 # v2 (future enhancements)
```

## Routing

The API version is extracted from the request path and normalized before routing:

1. Request arrives: `GET /api/v1/stats`
2. Version prefix stripped: normalized to `/api/stats`
3. Routed to handler
4. Response sent with appropriate version semantics

This approach allows:
- **100% backward compatibility** - All v0 endpoints still work
- **Forward compatibility** - New v2 features available without breaking v0/v1
- **Single implementation** - Handlers don't need to know about versions internally

## Version Differences

### v0/v1 - Stable APIs
All endpoints return the same response format. No breaking changes.

Response format example:
```json
{
  "status": "200 OK",
  "data": { /* version-specific data */ }
}
```

### v2 - Enhanced Features
For future releases, v2 endpoints may include:

- Additional response fields
- New filter/sort parameters
- Improved error messages with more context
- Enhanced validation

v2 API is **NOT** a breaking change - v0/v1 clients continue to work.

## Migration Path

### For API Clients

**Recommended migration path:**

1. **Current:** Use `/api/*` endpoints
2. **Short-term:** Switch to `/api/v1/*` for explicit versioning
3. **Future:** `/api/v2/*` when available with enhanced features

```bash
# Before
curl http://localhost:5030/api/stats

# After (recommended)
curl http://localhost:5030/api/v1/stats

# Future (when v2 features are available)
curl http://localhost:5030/api/v2/stats
```

### Implementation Details

Version normalization happens in `route_http_request_with_headers()`:

```rust
// Strip version prefix and track API version
let (normalized_path, api_version) = if path.starts_with("/api/v1/") {
    (path.replace("/api/v1/", "/api/"), 1)
} else if path.starts_with("/api/v2/") {
    (path.replace("/api/v2/", "/api/"), 2)
} else {
    (path, 0)  // v0/legacy
};
```

## Examples

### Health Check
```bash
curl http://localhost:5030/api/v1/health
# Response: {"status":"ok","service":"slskr"}
```

### Session Stats
```bash
curl -H "Authorization: Bearer token" \
  http://localhost:5030/api/v1/stats
# Response: { session stats JSON }
```

### Configuration
```bash
curl -H "Authorization: Bearer token" \
  http://localhost:5030/api/v1/config
# Response: { app configuration JSON }
```

## Testing

Version routing is tested to ensure all three paths work identically:

```bash
cargo test versioned_api_paths_map_to_current_handlers
```

Test case: `tests::versioned_api_paths_map_to_current_handlers`

## Future v2 Features

Planned enhancements for v2 API:

1. **GraphQL Support** - Query what you need, nothing more
   - Available at `POST /api/v2/graphql`
   - Replaces multiple REST endpoints

2. **Enhanced Webhooks** - More event types and filtering
   - Event filtering by type
   - Conditional delivery rules

3. **Batch Operations** - Send multiple requests in one call
   - `POST /api/v2/batch`
   - Reduces round-trips

4. **Server-Sent Events** - Real-time updates
   - `GET /api/v2/events/stream`
   - Browser-native event subscriptions

5. **Pagination** - Standardized cursor-based pagination
   - Better performance for large datasets
   - Consistent across all list endpoints

## Best Practices

1. **Use versioning explicitly** - Always include `/v1/` or `/v2/` in your URLs
2. **Don't hardcode paths** - Use API discovery endpoints
3. **Handle version deprecation** - Plan for v0 endpoints to sunset in major version 2.0
4. **Monitor deprecation headers** - v2 endpoints may include `Deprecation` headers

## Compatibility Matrix

| Feature | v0 | v1 | v2 |
|---------|----|----|-----|
| Core endpoints | ✓ | ✓ | ✓ |
| Webhooks | ✓ | ✓ | ✓+ |
| Rate limiting | ✓ | ✓ | ✓ |
| GraphQL | - | - | ✓ |
| Server-sent events | - | - | ✓ |
| Batch operations | - | - | ✓ |
| Cursor pagination | - | - | ✓+ |

✓ = Supported  
✓+ = Enhanced  
\- = Not available  
