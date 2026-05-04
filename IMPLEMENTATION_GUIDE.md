# POST Endpoints Implementation Guide - Quick Start

## Three Reference Documents Created

You now have three comprehensive guides:

1. **CODE_PATTERNS_ANALYSIS.md** - Deep dive into all store patterns, JSON extraction, SessionCommand, and response building
2. **FUNCTION_SIGNATURES_REFERENCE.md** - All function signatures with exact line numbers and copy-paste examples
3. **POST_ENDPOINTS_QUICK_REF.md** - Quick patterns for implementing each endpoint type
4. **This file** - Implementation checklist and summary

---

## Key Insights Summary

### 1. All Stores Follow This Pattern

```rust
// Lock the store
let mut store = state.STORE_NAME.write().await;

// Create/modify record
let record = store.create(...args...);

// Drop lock (very important!)
drop(store);

// Record event (optional but recommended)
record_event(state, "event.type", "category", details).await;

// Send command if needed (for network ops)
send_session_command(state, SessionCommand::...).await.ok();

// Return response
Ok(created_response(record.json()))
```

### 2. There Are Two JSON Extraction Functions You Need

**In main.rs:**
```rust
fn extract_json_u32_field(body: &str, field: &str) -> Option<u32>
fn extract_json_bool_field(body: &str, field: &str) -> Option<bool>
```

**In utils.rs (public):**
```rust
pub fn extract_json_string_field(body: &str, field: &str) -> Option<String>
pub fn extract_json_string_array_field(body: &str, field: &str) -> Option<Vec<String>>
```

**YOU MUST ADD:**
```rust
fn extract_json_u64_field(body: &str, field: &str) -> Option<u64>
```

This is required because TransferQueue::create() takes size as Option<u64>.

### 3. SessionCommand Enum Already Has Everything You Need

All the variants are already defined (lines 2377-2422):
- WatchUser(String)
- BrowseUser(String)
- JoinRoom(String)
- MessageUser { username: String, body: String }
- etc.

Just use: `send_session_command(state, SessionCommand::WatchUser(username)).await.ok();`

### 4. Response Types Are Pre-built

From routing.rs:
- `created_response(body)` → 201 Created
- `bad_request_response(msg)` → 400 Bad Request
- `not_found_response()` → 404 Not Found
- `conflict_response(msg)` → 409 Conflict (not exported but can be added)

---

## Step-by-Step Implementation Checklist

### For Each POST Endpoint:

- [ ] **Add route pattern** to match statement in `route_http_request_with_headers()` (line 2489+)
  ```rust
  ("POST", "/api/v0/path") => {
      // handler code
  }
  ```

- [ ] **Extract required JSON fields**
  ```rust
  let field = match extract_json_string_field(body, "field") {
      Some(f) => f,
      None => return Ok(bad_request_response("field required")),
  };
  ```

- [ ] **Extract optional JSON fields**
  ```rust
  let field = extract_json_u64_field(body, "field");
  let optional_field = extract_json_string_field(body, "optional").unwrap_or_default();
  ```

- [ ] **Validate inputs**
  ```rust
  if field.is_empty() {
      return Ok(bad_request_response("field cannot be empty"));
  }
  ```

- [ ] **Get mutable lock on store**
  ```rust
  let mut store = state.store_name.write().await;
  ```

- [ ] **Create/modify record**
  ```rust
  let record = store.create(field1, field2, field3);
  // OR
  let record = store.add(field1, field2);
  ```

- [ ] **Drop the lock**
  ```rust
  drop(store);
  ```

- [ ] **Record event (optional)**
  ```rust
  record_event(state, "event.type", "category", Some(format!("id={}", record.id))).await;
  ```

- [ ] **Send SessionCommand if needed (optional)**
  ```rust
  send_session_command(state, SessionCommand::WatchUser(username)).await.ok();
  ```

- [ ] **Return response**
  ```rust
  Ok(created_response(record.json()))
  ```

- [ ] **Add test case** (in #[cfg(test)] section)
  ```rust
  #[tokio::test]
  async fn test_post_endpoint() {
      let (state, _receiver) = test_state();
      let body = r#"{"field":"value"}"#;
      let response = route_http_request("POST", "/api/v0/path", None, body, &state)
          .await
          .expect("should work");
      assert_eq!(response.status, "201 Created");
  }
  ```

---

## Endpoints That Need Implementation

Based on test file analysis (test suite expects these):

### 1. POST /api/v0/transfers
- Create new transfer entry
- Stores: filename, direction, peer_username, local_path, size
- Returns: TransferEntry with status "queued"
- SessionCommand: None (transfer initiated via UI)

### 2. POST /api/v0/transfers/{id}/start
- Start an existing transfer
- Updates status from "queued" to "in_progress"
- SessionCommand: TransferPeer or IndirectTransfer

### 3. POST /api/v0/messages
- Create new message record
- Stores: username, direction, body
- SessionCommand: MessageUser

### 4. POST /api/v0/users/{username}/watch
- Watch a user
- SessionCommand: WatchUser

### 5. POST /api/v0/browse/{username}
- Request to browse user's shares
- SessionCommand: BrowseUser

### 6. POST /api/v0/rooms/{name}/join
- Join a room
- SessionCommand: JoinRoom

---

## Critical Code Locations

| Item | Location |
|------|----------|
| Route handler start | Line 2471 |
| Match statement | Line 2489 |
| extract_json_u32_field() | Line 2814 |
| extract_json_bool_field() | Line 2822 |
| send_session_command() | Line 4755 |
| AppState struct | Line 2361 |
| SessionCommand enum | Line 2377 |
| SearchStore::create() | Line 478 |
| TransferQueue::create() | Line 993 |
| MessageStore::add() | Line 2032 |
| UserStore::watch() | Line 1472 |
| RoomStore::join() | Line 2203 |
| BrowseStore::request() | Line 1705 |

---

## Common Mistakes to Avoid

1. **Forgetting to drop() the lock**
   ```rust
   // WRONG - lock held for entire function
   let mut store = state.store.write().await;
   let record = store.create(...);
   // ... lots of code here ...
   Ok(response)  // Lock still held!
   
   // RIGHT - drop lock immediately after use
   let mut store = state.store.write().await;
   let record = store.create(...);
   drop(store);
   // ... other code ...
   Ok(response)
   ```

2. **Not awaiting send_session_command()**
   ```rust
   // WRONG - async function not awaited
   send_session_command(state, SessionCommand::...).ok();
   
   // RIGHT - use await
   send_session_command(state, SessionCommand::...).await.ok();
   ```

3. **Returning Result instead of Option in extraction**
   ```rust
   // The extraction functions return Option, not Result
   // You must handle None explicitly
   let field = match extract_json_string_field(body, "field") {
       Some(f) => f,
       None => return Ok(bad_request_response("field required")),
   };
   ```

4. **Not escaping strings in JSON responses**
   ```rust
   // WRONG - unsanitized user input in JSON
   format!("{{\"message\":\"{}\"}}", user_input)
   
   // RIGHT - escape using json_escape()
   use crate::config::json_escape;
   format!("{{\"message\":\"{}\"}}", json_escape(user_input))
   ```

5. **Trying to use response functions that don't exist**
   ```rust
   // These exist:
   created_response(body)
   bad_request_response(msg)
   not_found_response()
   
   // This may not exist yet:
   conflict_response(msg)  // Check routing.rs
   
   // Add it if needed:
   pub fn conflict_response(message: &str) -> HttpResponse {
       HttpResponse {
           status: "409 Conflict",
           content_type: "application/json",
           body: format!("{{\"error\":\"{}\"}}", json_escape(message)),
       }
   }
   ```

---

## How SessionCommand.send() Works

```rust
// Async version - use this in async functions
async fn send_session_command(state: &AppState, command: SessionCommand) -> Result<(), String> {
    state.session_commands.send(command)
        .await
        .map_err(|_| "session manager is not running".to_owned())
}

// Usage:
send_session_command(state, SessionCommand::Connect).await?;

// Non-blocking version - use this for fire-and-forget
fn try_send_session_command(state: &AppState, command: SessionCommand) {
    let _ = state.session_commands.try_send(command);
}

// Usage:
try_send_session_command(state, SessionCommand::Disconnect);
```

The route handlers are async, so use the async version with `.await`.

---

## JSON Body Parsing Strategy

Always follow this pattern:

```rust
// 1. Extract field
// 2. Match on Option to handle None case
// 3. Return error early if validation fails
// 4. Use extracted value

let username = match extract_json_string_field(body, "username") {
    Some(u) if !u.is_empty() => u,  // <- Add validation here
    _ => return Ok(bad_request_response("username required")),
};

let size = extract_json_u64_field(body, "size");  // <- Optional, can be None

if let Some(size) = size {
    if size > 10_000_000_000 {  // 10GB limit
        return Ok(bad_request_response("file too large"));
    }
}
```

---

## Record Event Patterns

```rust
// Simple event
record_event(state, "transfer.created", "transfers", None).await;

// With details
record_event(
    state,
    "transfer.created",
    "transfers",
    Some(format!("id={},size={}", entry.id, entry.size.unwrap_or(0)))
).await;

// With escaped string
record_event(
    state,
    "user.watched",
    "users",
    Some(json_escape(&username))
).await;
```

---

## Complete Minimal Example

Here's a complete working POST handler:

```rust
("POST", "/api/v0/messages") => {
    // Extract required fields
    let username = match extract_json_string_field(body, "username") {
        Some(u) => u,
        None => return Ok(bad_request_response("username required")),
    };
    
    let body_text = match extract_json_string_field(body, "body") {
        Some(b) => b,
        None => return Ok(bad_request_response("body required")),
    };
    
    // Validate
    if username.is_empty() {
        return Ok(bad_request_response("username cannot be empty"));
    }
    
    if body_text.is_empty() {
        return Ok(bad_request_response("body cannot be empty"));
    }
    
    // Create record
    let mut messages = state.messages.write().await;
    let record = messages.add(username.clone(), "in", body_text);
    drop(messages);
    
    // Record event
    record_event(
        state,
        "message.received",
        "messages",
        Some(format!("id={},from={}", record.id, json_escape(&username)))
    ).await;
    
    // Return response
    Ok(created_response(record.json()))
}
```

---

## Next Steps

1. Read CODE_PATTERNS_ANALYSIS.md for deep understanding
2. Use FUNCTION_SIGNATURES_REFERENCE.md as copy-paste reference
3. Follow POST_ENDPOINTS_QUICK_REF.md for each endpoint
4. Use this checklist to verify completeness
5. Test each endpoint with test cases
6. Run `cargo test` to verify all tests pass

