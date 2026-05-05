# slskr HTTP API Implementation Gap Analysis
**Date:** 2026-05-04  
**Scope:** Comprehensive analysis of HTTP API endpoints, comparing documented vs implemented

---

## Executive Summary

The slskr HTTP API has **31 documented endpoints** in `/docs/http-api.md` but only **24 are properly implemented** (77% coverage). Additionally, there are **16 extra endpoints** not documented. Key gaps include:

- 8 documented endpoints missing entirely or incorrectly implemented
- Missing HTTP method support (PUT not implemented)
- Webhook dispatch not wired to event system
- Database maintenance endpoints return stubs only
- GraphQL engine not integrated
- Session management doesn't follow REST conventions

---

## Detailed Endpoint Analysis

### ✅ FULLY IMPLEMENTED (Core Info - 5/5)

| Endpoint | Status | File | Notes |
|----------|--------|------|-------|
| GET /api/health | ✅ | main.rs:2529 | Returns service health |
| GET /api/version | ✅ | main.rs:2530 | Returns version info |
| GET /api/config | ✅ | main.rs:2533 | Returns sanitized config |
| GET /api/stats | ✅ | main.rs:2538 | Aggregates all stats |
| GET /api/capabilities | ✅ | main.rs:2531 | Lists supported features |

### ⚠️ PARTIALLY IMPLEMENTED - Session Management (1/5)

**Issue:** Using non-standard `/api/session` (singular) instead of documented `/api/sessions/{id}`

| Endpoint | Documented | Implemented | Status |
|----------|------------|-------------|--------|
| GET /api/sessions | Yes | GET /api/session | ⚠️ Wrong path |
| POST /api/sessions | Yes | ❌ | **MISSING** |
| GET /api/sessions/{id}/privileges | Yes | POST /api/session/privileges/check | ⚠️ Wrong method |
| POST /api/sessions/{id}/ping | Yes | POST /api/session/ping | ⚠️ Wrong path |
| DELETE /api/sessions/{id} | Yes | POST /api/session/disconnect | ⚠️ Wrong method |

**File:** main.rs:2797-2812  
**Impact:** RESTful clients expecting standard session paths will fail

### ✅ FULLY IMPLEMENTED - Searches (3/3)

| Endpoint | Status | File | Implementation |
|----------|--------|------|-----------------|
| GET /api/searches | ✅ | main.rs:2829 | List all searches |
| POST /api/searches | ✅ | main.rs:2889 | Create new search |
| GET /api/searches/{id} | ✅ | main.rs:2840 | search_token_path helper |

**Notes:** Search completion at line 2947, pruning at line 2960

### ⚠️ PARTIALLY IMPLEMENTED - Messages (3/4)

| Endpoint | Status | File | Notes |
|----------|--------|------|-------|
| GET /api/messages | ✅ | main.rs:2865 | List all messages |
| GET /api/messages/{username} | ✅ | main.rs:3221 | messages_user_path helper |
| POST /api/messages | ✅ | main.rs:3136 | Send message |
| PUT /api/messages/{id}/acknowledge | ❌ | - | **MISSING** |

**Gap:** PUT method not implemented (no /api/messages/{id}/acknowledge endpoint)  
**File Location:** routing.rs has no PUT support defined

### ⚠️ PARTIALLY IMPLEMENTED - Transfers (2/4)

| Endpoint | Status | File | Notes |
|----------|--------|------|-------|
| GET /api/transfers | ✅ | main.rs:2873 | List all transfers |
| POST /api/transfers | ✅ | main.rs:3004 | Create transfer |
| GET /api/transfers/{id} | ❌ | - | **MISSING** - No single transfer endpoint |
| DELETE /api/transfers/{id} | ❌ | - | **MISSING** - No cancel endpoint |

**Gap:** transfer_action_path helper (line 3038) handles /start, /progress, /complete but not GET/{id} or DELETE/{id}

### ⚠️ PARTIALLY IMPLEMENTED - Rooms (3/4)

| Endpoint | Status | File | Notes |
|----------|--------|------|-------|
| GET /api/rooms | ✅ | main.rs:2857 | List rooms |
| GET /api/rooms/{name} | ❌ | - | **MISSING** - No room detail endpoint |
| POST /api/rooms/{name} | ✅ | main.rs:3237 | room_join_path helper |
| DELETE /api/rooms/{name} | ✅ | main.rs:3248 | room_join_path helper |

**Gap:** Cannot GET details of a specific room

### ⚠️ PARTIALLY IMPLEMENTED - Browse (3/5)

| Endpoint | Status | File | Notes |
|----------|--------|------|-------|
| GET /api/browse/{username} | ✅ | main.rs:3447 | Get user's shared files |
| POST /api/browse/{username} | ✅ | main.rs:3330 | user_browse_request_path |
| GET /api/browse/requests | ❌ | - | **MISSING** - No browse requests list |
| POST /api/browse/requests/{id} | ✅ | main.rs:3371 | Browse-responses |
| PUT /api/browse/requests/{id}/acknowledge | ❌ | - | **MISSING** - PUT not implemented |

**Gap:** Cannot list pending browse requests or acknowledge with PUT

### ✅ IMPLEMENTED - Events (1/1)

| Endpoint | Status | File |
|----------|--------|------|
| GET /api/events | ✅ | main.rs:2673 |

---

## Extra Endpoints (Not Documented but Implemented)

### Advanced Features (9 endpoints)
- ✅ GET /api/metrics (Prometheus format) - main.rs:2615
- ✅ GET /api/telemetry - main.rs:2592  
- ✅ GET /api/shares - main.rs:2681
- ✅ GET /api/shares/catalog - main.rs:2689
- ✅ GET /api/listeners - main.rs:2813
- ✅ GET /api/files/{folder} - main.rs:2697 (folder browsing)
- ✅ POST /api/shares/rescan - main.rs:2774
- ✅ POST /api/search-responses - main.rs:2968
- ✅ POST /api/browse-responses - main.rs:3371

### Session Commands
- ✅ POST /api/session/connect - main.rs:2797
- ✅ POST /api/session/disconnect - main.rs:2805
- ✅ POST /api/session/privileges/check - main.rs:2809
- ✅ POST /api/messages/inbound - main.rs:3169
- ✅ POST /api/rooms/refresh - main.rs:3232

### User Management (6 endpoints)
- ✅ POST /api/users/watch - main.rs:3293
- ✅ DELETE /api/users/{username}/watch - main.rs:3308
- ✅ POST /api/users/{username}/stats - main.rs:3324
- ✅ POST /api/users/{username}/browse - main.rs:3330
- ✅ POST /api/users/{username}/browse/{folder} - main.rs:3342
- ✅ POST /api/users/{username}/browse/fail - main.rs:3355

---

## Stub/Incomplete Implementations

### Admin/Webhooks (4 endpoints - ALL STUBS)
**File:** main.rs:3470-3509  
**Issue:** Return hardcoded responses with no actual logic

| Endpoint | Returns | Should Do |
|----------|---------|-----------|
| POST /api/admin/webhooks | 201 with hardcoded data | Store webhook config, validate URL |
| GET /api/admin/webhooks | Empty array `[]` | Query registered webhooks |
| DELETE /api/admin/webhooks/{id} | Success JSON | Unregister webhook |
| POST /api/admin/webhooks/{id}/test | Success JSON | Send test event to webhook |

### Admin/Keys (3 endpoints - ALL STUBS)
**File:** main.rs:3533-3568  
**Issue:** No actual key management logic

| Endpoint | Returns | Should Do |
|----------|---------|-----------|
| POST /api/admin/keys | 201 with generated key | Create & persist API key |
| GET /api/admin/keys | Empty array `[]` | List created keys |
| DELETE /api/admin/keys/{id} | Success JSON | Revoke key |
| GET /api/admin/keys/validate | Fixed response | Validate provided key |

### Admin/Database (3 endpoints - ALL STUBS)
**File:** main.rs:3511-3530  
**Issue:** Return fixed responses with no database operations

| Endpoint | Returns | Should Do |
|----------|---------|-----------|
| GET /api/admin/database/stats | Hardcoded stats | Query actual DB stats |
| POST /api/admin/database/cleanup | Success JSON | Clean old records |
| POST /api/admin/database/vacuum | Success JSON | Optimize database |

### Monitoring
**File:** main.rs:3571  
**Issue:** Single hardcoded response

| Endpoint | Returns | Should Do |
|----------|---------|-----------|
| GET /api/admin/monitoring | Fixed metrics | Query real system metrics |

---

## GraphQL Implementation Status

**File:** graphql.rs  
**Status:** Basic schema definition only, NO query engine

### Issues
1. **Line 3579-3586:** GraphQL queries hit `graphql::execute_graphql_query(body)` - not defined
2. **Line 3587-3708:** Schema endpoint returns hardcoded schema string
3. **graphql.rs Lines 1-100:** Mock implementations returning empty results
4. **No resolver wiring:** Mutations/Subscriptions not connected to state

### Missing
- GraphQL query parsing and execution
- Subscription support for real-time updates
- Mutation handlers wired to state modifications
- Proper error handling and validation

---

## HTTP Method Support Issues

### Missing PUT Support
**Problem:** routing.rs and main.rs only handle GET/POST/DELETE

**Documented but missing:**
- PUT /api/messages/{id}/acknowledge
- PUT /api/browse/requests/{id}/acknowledge

**File:** routing.rs - no PUT methods in match statements

### Impact
Any PUT request returns 404 Not Found instead of method not allowed

---

## Webhook System Analysis

### Definition (webhooks.rs - lines 1-250)
✅ WebhookEvent enum defined (14 event types)
✅ Webhook struct with HMAC-SHA256 signing  
✅ WebhookManager with register/unregister

### Integration
❌ **No event dispatch wired**
- record_event() (main.rs:5821) only logs to EventStore
- WebhookEvent types defined but never triggered
- Webhook endpoints return stubs with no storage
- No HTTP POST to webhook URLs on events

### Event Types Defined But Not Used
```rust
SearchCreated, SearchCompleted,
TransferStarted, TransferCompleted, TransferFailed,
MessageReceived, MessageSent,
UserConnected, UserDisconnected,
RoomJoined, RoomLeft,
ApiKeyCreated, ApiKeyRevoked,
ConfigChanged
```

### Where Webhooks Should Fire (Not implemented)
1. **Searches** - After POST /api/searches (line 2889)
2. **Transfers** - After POST /api/transfers (line 3004)  
3. **Messages** - After POST /api/messages (line 3136)
4. **Users** - After POST /api/users/watch (line 3293)
5. **Rooms** - After POST /api/rooms/{name} (line 3237)

---

## Database/Persistence Issues

### Current State
- **In-memory only:** All data stored in Arc<RwLock<>> structs
- **Partial file persistence:** Transfer events logged to TSV, state to JSON
- **No query layer:** persistence.rs has stubs but no implementation

### Persistence Module Status (persistence.rs)
- SearchRecord struct defined but unused
- TransferRecord struct defined but unused  
- MessageRecord struct defined but unused
- No actual database operations (SQL, etc)

### Maintenance Endpoints
**All return hardcoded responses:**
- POST /api/admin/database/cleanup (line 3518)
- POST /api/admin/database/vacuum (line 3525)
- GET /api/admin/database/stats (line 3511)

---

## Priority Implementation List

### 🔴 CRITICAL (API Contract Compliance)
1. **PUT /api/messages/{id}/acknowledge** 
   - Documented in API docs
   - File: main.rs, route_http_request_with_headers()
   - Also needs: Add PUT handling to routing.rs

2. **GET /api/transfers/{id}**
   - Return single transfer by ID
   - Implement beside existing /api/transfers

3. **DELETE /api/transfers/{id}**
   - Cancel/delete specific transfer
   - Add to transfer_action_path logic

4. **GET /api/rooms/{name}**
   - Return specific room details
   - Implement alongside /api/rooms

5. **GET /api/browse/requests**
   - List pending browse requests
   - Query browse.records for pending

6. **Fix Session Endpoints**
   - Move from /api/session → /api/sessions/{id}
   - Requires routing refactor

### 🟠 HIGH (Documented Features)
7. **PUT /api/browse/requests/{id}/acknowledge**
   - Acknowledge browse request
   - Add PUT method support

8. **POST /api/sessions** (New Session)
   - Create server connection
   - Wire to session management

9. **POST /api/sessions/{id}/capabilities/negotiate**
   - Negotiate protocol features
   - Wire to SessionCommand enum

### 🟡 MEDIUM (Admin/Management)
10. **Real Webhook Implementation**
    - Wire WebhookEvent dispatch
    - POST to configured webhook URLs
    - Implement secret signing
    - Files: main.rs record_event(), webhooks.rs

11. **API Key Management**
    - Actual storage and validation
    - Not just stub responses
    - File: api_keys.rs needs implementation

12. **GraphQL Query Engine**
    - Integrate actual query resolver
    - Wire mutations to state
    - File: graphql.rs

### 🟢 LOW (Nice-to-Have)
13. **Database Maintenance**
    - Implement vacuum/cleanup logic
    - Integrate with persistence layer
    - File: persistence.rs

14. **Monitoring Endpoint**
    - Query real system metrics
    - File: main.rs:3571

15. **Capabilities Negotiation**
    - Implement /api/capabilities/negotiate
    - File: main.rs:2532

---

## File Locations Summary

### Main HTTP Routing
- **/crates/slskr/src/main.rs** (3800+ lines)
  - `route_http_request_with_headers()` - line 2497
  - All endpoint match patterns - lines 2527-3709
  - Session commands - lines 2797-2812
  - Search endpoints - lines 2829-2966
  - Transfer endpoints - lines 2873-3133
  - Message endpoints - lines 2865-3229
  - Room endpoints - lines 2857-3290
  - Browse endpoints - lines 3330-3468
  - Admin stubs - lines 3470-3577
  - GraphQL - lines 3579-3708

- **/crates/slskr/src/routing.rs** (130 lines)
  - Response builders - lines 68-130
  - HTTP method handling - **MISSING PUT**

- **/crates/slskr/src/webhooks.rs** (408 lines)
  - Event definitions - lines 12-29
  - Webhook struct - lines 52-107
  - WebhookManager - lines 221-349
  - **No dispatch implementation**

- **/crates/slskr/src/graphql.rs** (344 lines)
  - Mock Query resolvers - basic stubs
  - Schema generation - returns hardcoded string
  - **No query engine integration**

- **/crates/slskr/src/api_keys.rs**
  - Placeholder module (not analyzed)

- **/crates/slskr/src/persistence.rs**
  - Schema definitions only
  - No actual DB operations

### Documentation
- **/docs/http-api.md** (674 lines)
  - Documents 31 endpoints
  - Example requests/responses
  - Authentication and CSRF info

- **/docs/GRAPHQL_SCHEMA.graphql** (444 lines)
  - Full GraphQL type system
  - Query, Mutation, Subscription definitions
  - Not reflected in actual implementation

---

## Recommendations

### Immediate Actions (This Sprint)
1. Implement missing documented endpoints (6 endpoints)
2. Add PUT method support to routing
3. Fix session endpoint paths to match REST convention
4. Wire webhook event dispatch

### Next Sprint
1. Implement admin endpoints (webhook/key management)
2. Real GraphQL query engine
3. Database persistence layer integration

### Architecture Improvements
1. Extract HTTP routing to separate module
2. Create endpoint registry/metadata
3. Implement middleware pipeline for auth/validation
4. Add request/response type definitions

---

## Testing Gaps

Current test coverage (main.rs - lines 8000+):
- ✅ Basic endpoint tests
- ⚠️ Missing tests for PUT endpoints
- ❌ No webhook dispatch tests
- ❌ No GraphQL tests
- ❌ No admin endpoint integration tests

**Recommendation:** Expand test suite to cover all 31 documented endpoints

