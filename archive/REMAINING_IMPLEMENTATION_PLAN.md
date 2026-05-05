# slskr Remaining Implementation Plan

**Status:** Phase 8 (Hardening) - Core functionality complete, finishing gaps

**Last Updated:** 2026-05-04

## Summary of Work Completed This Session

### ✅ Durable Storage Layer (SQLite)
- Added `sqlx` 0.7 with SQLite backend to dependencies
- Implemented full `DatabaseManager` with async API
- Created schema for:
  - `searches` table with indices
  - `transfers` table with progress tracking
  - `messages` table with read status
  - `user_stats` table for watched users
  - `rooms` table for subscriptions
- Operations:
  - Full CRUD for searches, transfers, messages
  - User statistics tracking (uploads/downloads)
  - Room subscription management
  - Database maintenance (cleanup old records, vacuum)
  - Comprehensive test suite (6 passing tests)
- Integrated `DatabaseManager` into `AppState`
- All 151 existing unit tests passing

### Current State
- SQLite layer fully functional and tested
- Ready for endpoint wiring
- Database path: `~/.local/state/slskr/slskr.db`

---

## Remaining Work (Priority Order)

### Priority 1: Critical API Gaps (HIGH VALUE, MEDIUM EFFORT)

#### 1.1 PUT Method Support
**Files:** `crates/slskr/src/main.rs` (line ~2527)
**Effort:** ~2 hours

Required for:
- `PUT /api/messages/{id}/acknowledge` - Mark message as read
- `PUT /api/browse/requests/{id}/acknowledge` - Acknowledge browse request

**Implementation:**
```rust
// Add to route match statement:
("PUT", path) if path.starts_with("/api/messages/") => {
    let id = extract_id_from_path(path, "/api/messages/");
    let mut messages = state.messages.write().await;
    if messages.mark_as_read(id) {
        Ok(ok_response(json!({ "id": id, "read": true })))
    } else {
        Ok(not_found_response())
    }
}

("PUT", path) if path.starts_with("/api/browse/requests/") => {
    let id = extract_id_from_path(path, "/api/browse/requests/");
    let mut browse = state.browse.write().await;
    if browse.acknowledge_request(id) {
        Ok(ok_response(json!({ "id": id, "acknowledged": true })))
    } else {
        Ok(not_found_response())
    }
}
```

#### 1.2 Database Maintenance Endpoints
**Files:** `crates/slskr/src/main.rs` (new routes)
**Effort:** ~1 hour
**Value:** Admin tools, data management

Routes to implement:
- `GET /api/v0/database/stats` - Database statistics (uses new `db.get_stats()`)
- `POST /api/v0/database/cleanup` - Clean old records (uses `db.cleanup_old_records()`)
- `POST /api/v0/database/vacuum` - Optimize storage (uses `db.vacuum()`)

**Implementation:**
```rust
("GET", "/api/v0/database/stats") => {
    if let Some(ref db) = state.db {
        match db.get_stats().await {
            Ok(stats) => Ok(ok_response(serde_json::to_string(&stats)?)),
            Err(e) => Ok(conflict_response(&e.to_string())),
        }
    } else {
        Ok(conflict_response("database not initialized"))
    }
}

("POST", "/api/v0/database/cleanup") => {
    let days: i32 = extract_json_i32_field(body, "days").unwrap_or(30);
    if let Some(ref db) = state.db {
        match db.cleanup_old_records(days).await {
            Ok(count) => Ok(ok_response(json!({ "cleaned": count, "days": days }))),
            Err(e) => Ok(conflict_response(&e.to_string())),
        }
    } else {
        Ok(conflict_response("database not initialized"))
    }
}

("POST", "/api/v0/database/vacuum") => {
    if let Some(ref db) = state.db {
        match db.vacuum().await {
            Ok(_) => Ok(ok_response(json!({ "vacuumed": true }))),
            Err(e) => Ok(conflict_response(&e.to_string())),
        }
    } else {
        Ok(conflict_response("database not initialized"))
    }
}
```

#### 1.3 Transfer Detail Endpoints
**Files:** `crates/slskr/src/main.rs`
**Effort:** ~1 hour

Routes:
- `GET /api/v0/transfers/{id}` - Get single transfer details
- `DELETE /api/v0/transfers/{id}` - Cancel transfer
- `PUT /api/v0/transfers/{id}/complete` - Mark as complete

**Implementation Pattern:**
```rust
("GET", path) if path.starts_with("/api/v0/transfers/") && path.matches("/").count() == 3 => {
    let id = extract_id_from_path(path, "/api/v0/transfers/");
    let transfers = state.transfers.read().await;
    if let Some(transfer) = transfers.get_by_id(id) {
        Ok(ok_response(transfer.to_json()))
    } else {
        Ok(not_found_response())
    }
}

("DELETE", path) if path.starts_with("/api/v0/transfers/") && path.matches("/").count() == 3 => {
    let id = extract_id_from_path(path, "/api/v0/transfers/");
    let mut transfers = state.transfers.write().await;
    if transfers.cancel(id) {
        Ok(ok_response(json!({ "cancelled": true })))
    } else {
        Ok(not_found_response())
    }
}
```

---

### Priority 2: Browse/Message/Room Completeness (MEDIUM VALUE, MEDIUM EFFORT)

#### 2.1 Room Detail Endpoints
**Files:** `crates/slskr/src/main.rs`
**Effort:** ~1.5 hours

Routes:
- `GET /api/v0/rooms/{name}` - Get room details
- `GET /api/v0/rooms/{name}/users` - List users in room
- `POST /api/v0/rooms/{name}/join` - Join room
- `POST /api/v0/rooms/{name}/leave` - Leave room

#### 2.2 Browse Request Management
**Files:** `crates/slskr/src/main.rs`
**Effort:** ~1 hour

Routes:
- `GET /api/v0/browse/requests` - List pending browse requests
- `POST /api/v0/browse/requests/{id}/accept` - Accept browse
- `POST /api/v0/browse/requests/{id}/reject` - Reject browse

#### 2.3 Message Operations
**Files:** `crates/slskr/src/main.rs`
**Effort:** ~1 hour

Routes:
- `GET /api/v0/messages/{id}` - Get single message
- `DELETE /api/v0/messages/{id}` - Delete message
- `PUT /api/v0/messages/{id}/mark-read` - Mark message read

---

### Priority 3: Webhook Event Dispatch (HIGH VALUE, HIGH EFFORT)

**Files:** `crates/slskr/src/webhooks.rs`, `crates/slskr/src/main.rs`
**Effort:** ~4-6 hours
**Value:** Real-time event notifications, external integrations

#### 3.1 Webhook Dispatcher Architecture
- Create `WebhookDispatcher` struct in webhooks module
- Add async webhook sending with retries
- Integrate into session manager and transfer queue
- HTTP client for actual webhook POST requests

#### 3.2 Event Triggers
Need to call webhook dispatch at these points:
- Search created/completed (in session manager)
- Transfer started/completed/failed (in transfer queue)
- Message received/sent (in message handler)
- User connected/disconnected (in session events)
- Room joined/left (in room manager)
- API key created/revoked (in API key handler)
- Config changed (in config handler)

#### 3.3 Implementation Strategy
```rust
// Add webhook dispatcher to AppState
pub struct WebhookDispatcher {
    client: reqwest::Client,  // For HTTPS requests
    sender: mpsc::Sender<WebhookTask>,
}

enum WebhookTask {
    Trigger { event: WebhookEvent, payload: serde_json::Value },
    Retry { webhook_id: String, attempt: u32 },
}

// Spawn background task to handle webhook sends
```

---

### Priority 4: Health/Diagnostics Endpoints (LOW EFFORT, MEDIUM VALUE)

**Files:** `crates/slskr/src/main.rs`
**Effort:** ~2 hours

Routes:
- `GET /api/v0/health/detailed` - Expanded health check with system info
- `GET /api/v0/diagnostics` - System diagnostics (memory, connection count, etc)
- `GET /api/v0/diagnostics/transfers` - Transfer queue diagnostics
- `GET /api/v0/diagnostics/peers` - Connected peers diagnostics

---

### Priority 5: GraphQL Endpoint Wiring (MEDIUM EFFORT, MEDIUM VALUE)

**Files:** `crates/slskr/src/graphql.rs`, `crates/slskr/src/main.rs`
**Effort:** ~3-4 hours

Current state: Schema defined but no query execution

Options:
1. **Lightweight approach:** Implement simple GraphQL query parser for key queries
2. **Heavy approach:** Integrate `async-graphql` or `juniper` crate

Recommended: Lightweight approach
- Implement `SearchQuery`, `TransferQuery`, `UserQuery` resolvers
- Map to existing in-memory stores
- Support common filter/pagination patterns

---

### Priority 6: Public Release Preparation (MEDIUM EFFORT)

**Files:** Multiple, especially docs and repo config
**Effort:** ~2-3 hours

Checklist:
- [ ] Update LICENSE headers in all source files
- [ ] Create SECURITY.md for vulnerability reporting
- [ ] Add CONTRIBUTING.md for open source guidelines
- [ ] Review and update all API documentation
- [ ] Add CHANGELOG.md with release notes
- [ ] Register on crates.io and publish
- [ ] Create GitHub Actions CI/CD for releases
- [ ] Add badges to README (build status, crates.io, license)
- [ ] Verify no hardcoded secrets in code/commits
- [ ] Review API for any breaking changes before 1.0

---

## Integration Testing Strategy

### Test Type-1 Obfuscation with slskr
**Command:**
```bash
# Terminal 1: Start slskr
cargo run -p slskr -- serve

# Terminal 2: Start slskr (third-party client)
slskr --config /tmp/slskr-config.json

# Terminal 3: Run interop tests
scripts/run-live-soak-proton-natpmp.sh  # Or custom test script
```

### Test Coverage Needed
- [ ] Type-1 obfuscated peer connection (P stream)
- [ ] File transfer with obfuscation
- [ ] Search dispatch and response
- [ ] Distributed search tree
- [ ] File listing browse
- [ ] Private messages
- [ ] User stats requests

---

## Phase 8 (Hardening) Completion Criteria

- [x] All 151 unit tests passing
- [x] Durable storage (SQLite) working
- [x] Database schema complete and indexed
- [ ] All critical API gaps filled (PUT, database endpoints)
- [ ] Webhook dispatch working
- [ ] Type-1 obfuscation interop verified with slskr
- [ ] 24-hour stability soak test passing
- [ ] Health check and diagnostics endpoints
- [ ] GraphQL endpoint wired (or documentation of planned approach)
- [ ] Public release documentation complete

---

## Estimated Timeline

| Priority | Task | Effort | Est. Hours |
|----------|------|--------|-----------|
| 1.1 | PUT Method Support | Medium | 2 |
| 1.2 | Database Maintenance Endpoints | Medium | 1 |
| 1.3 | Transfer Detail Endpoints | Medium | 1 |
| 2.1-2.3 | Browse/Message/Room Completion | Medium | 3.5 |
| 3.1-3.3 | Webhook Event Dispatch | High | 5 |
| 4 | Health/Diagnostics | Low | 2 |
| 5 | GraphQL Wiring | Medium | 3 |
| 6 | Public Release Prep | Medium | 3 |
| **Total** | | | **~20.5 hours** |

---

## Success Metrics

1. **API Completeness:** 100% of documented endpoints implemented
2. **Test Coverage:** All existing tests pass + new endpoint tests
3. **Performance:** 24-hour soak test with <1% error rate
4. **Compatibility:** Full interop with slskr/slskr clients
5. **Reliability:** No memory leaks, graceful error handling
6. **Documentation:** All endpoints documented with examples

---

## Next Steps (Immediate)

1. **Commit current state** (SQLite implementation + tests)
2. **Implement Priority 1 endpoints** (PUT, database, transfer detail)
3. **Wire webhook dispatch** (high impact feature)
4. **Run integration tests** with slskr
5. **Complete Phase 8 validation** and begin public release prep

