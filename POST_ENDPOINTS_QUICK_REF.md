# POST Endpoints Implementation Quick Reference

## Step-by-Step Pattern for Each Endpoint Type

### 1. POST /api/v0/transfers (Create Transfer)

**Expected JSON:**
```json
{
  "direction": 0|1,
  "filename": "string",
  "peer_username": "string" (optional),
  "local_path": "string" (optional),
  "size": number (optional)
}
```

**Implementation:**
```rust
("POST", "/api/v0/transfers") => {
    let filename = match extract_json_string_field(body, "filename") {
        Some(f) => f,
        None => return Ok(bad_request_response("filename required")),
    };
    
    let direction = extract_json_u32_field(body, "direction").unwrap_or(1);
    let size = extract_json_u64_field(body, "size");
    let peer_username = extract_json_string_field(body, "peer_username");
    let local_path = extract_json_string_field(body, "local_path");
    
    let mut transfers = state.transfers.write().await;
    let entry = transfers.create(direction, peer_username, filename, local_path, size);
    drop(transfers);
    
    Ok(created_response(entry.json()))
}
```

---

### 2. POST /api/v0/transfers/{id}/start (Start Transfer)

**Implementation:**
```rust
("POST", path) if transfer_start_path(route.normalized_path).is_some() => {
    let id = transfer_start_path(route.normalized_path).unwrap();
    
    // Get transfer status
    let mut transfers = state.transfers.write().await;
    if let Some(entry) = transfers.get_mut(id) {
        // Validation logic here
        entry.status = "in_progress".to_owned();
        drop(transfers);
        
        Ok(response_with_status_code("200 OK", entry.json()))
    } else {
        drop(transfers);
        Ok(not_found_response())
    }
}
```

---

### 3. POST /api/v0/messages (Create Message)

**Expected JSON:**
```json
{
  "username": "string",
  "direction": "in"|"out",
  "body": "string"
}
```

**Implementation:**
```rust
("POST", "/api/v0/messages") => {
    let username = match extract_json_string_field(body, "username") {
        Some(u) => u,
        None => return Ok(bad_request_response("username required")),
    };
    
    let direction = match extract_json_string_field(body, "direction") {
        Some(d) if d == "in" || d == "out" => if d == "in" { "in" } else { "out" },
        _ => return Ok(bad_request_response("direction must be 'in' or 'out'")),
    };
    
    let body_text = match extract_json_string_field(body, "body") {
        Some(b) => b,
        None => return Ok(bad_request_response("body required")),
    };
    
    let mut messages = state.messages.write().await;
    let record = messages.add(username, direction, body_text);
    drop(messages);
    
    record_event(state, "message.created", "messages", Some(format!("id={}", record.id))).await;
    
    Ok(created_response(record.json()))
}
```

---

### 4. POST /api/v0/users/{username}/watch (Watch User)

**Implementation:**
```rust
("POST", path) if user_watch_path(route.normalized_path).is_some() => {
    let username = user_watch_path(route.normalized_path).unwrap();
    
    let mut users = state.users.write().await;
    let record = users.watch(username.to_string());
    drop(users);
    
    record_event(state, "user.watched", "users", Some(username)).await;
    send_session_command(state, SessionCommand::WatchUser(username.to_string())).await.ok();
    
    Ok(created_response(record.json()))
}
```

---

### 5. POST /api/v0/browse/{username} (Browse User)

**Implementation:**
```rust
("POST", path) if browse_user_path(route.normalized_path).is_some() => {
    let username = browse_user_path(route.normalized_path).unwrap();
    
    let mut browse = state.browse.write().await;
    let record = browse.request(username.to_string());
    drop(browse);
    
    record_event(state, "browse.requested", "browse", Some(username)).await;
    send_session_command(state, SessionCommand::BrowseUser(username.to_string())).await.ok();
    
    Ok(created_response(record.json()))
}
```

---

### 6. POST /api/v0/rooms/{name}/join (Join Room)

**Implementation:**
```rust
("POST", path) if room_join_path(route.normalized_path).is_some() => {
    let room_name = room_join_path(route.normalized_path).unwrap();
    
    let mut rooms = state.rooms.write().await;
    let record = rooms.join(room_name.to_string());
    drop(rooms);
    
    record_event(state, "room.joined", "rooms", Some(room_name)).await;
    send_session_command(state, SessionCommand::JoinRoom(room_name.to_string())).await.ok();
    
    Ok(created_response(record.json()))
}
```

---

## Path Extraction Helper Function Pattern

Create these helper functions to extract path parameters:

```rust
fn transfer_start_path(normalized_path: &str) -> Option<u64> {
    if normalized_path.starts_with("/api/v0/transfers/") && normalized_path.ends_with("/start") {
        let middle = normalized_path.strip_prefix("/api/v0/transfers/")?
            .strip_suffix("/start")?;
        middle.parse().ok()
    } else {
        None
    }
}

fn user_watch_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/v0/users/") && normalized_path.ends_with("/watch") {
        normalized_path.strip_prefix("/api/v0/users/")?
            .strip_suffix("/watch")
            .map(|s| s)
    } else {
        None
    }
}

fn browse_user_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/v0/browse/") {
        normalized_path.strip_prefix("/api/v0/browse/")
    } else {
        None
    }
}

fn room_join_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/v0/rooms/") && normalized_path.ends_with("/join") {
        normalized_path.strip_prefix("/api/v0/rooms/")?
            .strip_suffix("/join")
    } else {
        None
    }
}
```

---

## Common Imports Needed

Add these to the top of main.rs if not already present:

```rust
use crate::utils::extract_json_string_field;
use crate::config::json_escape;
use routing::bad_request_response;
```

---

## Testing Each Endpoint

### In test section:
```rust
#[tokio::test]
async fn test_create_transfer() {
    let (state, _receiver) = test_state();
    
    let body = r#"{"filename":"test.mp3","size":5000}"#;
    let response = route_http_request("POST", "/api/v0/transfers", None, body, &state)
        .await
        .expect("should succeed");
    
    assert_eq!(response.status, "201 Created");
    assert!(response.body.contains("\"id\":1"));
}
```

---

## Response Body Format

Each resource should have a `.json()` method that returns formatted JSON. Ensure records implement:

```rust
impl SearchRecord {
    pub fn json(&self) -> String {
        format!(
            "{{\"token\":{},\"query\":\"{}\",\"target\":\"{}\",\"status\":\"{}\",...}}",
            self.token,
            json_escape(&self.query),
            self.target,
            self.status
        )
    }
}
```

---

## Error Handling Patterns

**Required field missing:**
```rust
return Ok(bad_request_response("field_name required"));
```

**Invalid value format:**
```rust
return Ok(bad_request_response("field_name must be valid number"));
```

**Resource not found:**
```rust
return Ok(not_found_response());
```

**Business logic constraint:**
```rust
return Ok(conflict_response("constraint description"));
```

---

## Key Functions to Import/Use

| Function | Source | Usage |
|----------|--------|-------|
| `extract_json_string_field()` | utils.rs | Extract string fields from JSON |
| `extract_json_u32_field()` | main.rs | Extract u32 fields from JSON |
| `extract_json_u64_field()` | main.rs | Extract u64 fields from JSON |
| `extract_json_bool_field()` | main.rs | Extract boolean fields from JSON |
| `json_escape()` | config.rs | Escape strings for JSON output |
| `bad_request_response()` | routing.rs | 400 error response |
| `not_found_response()` | routing.rs | 404 error response |
| `conflict_response()` | routing.rs | 409 error response |
| `created_response()` | routing.rs | 201 created response |
| `send_session_command()` | main.rs | Send command to session manager |
| `record_event()` | main.rs | Log event to audit trail |
| `unix_timestamp()` | utils.rs | Get current UNIX timestamp |

---

## Checklist for Each New POST Endpoint

- [ ] Add route pattern to match statement in `route_http_request_with_headers()`
- [ ] Extract JSON fields with proper error handling
- [ ] Validate required fields
- [ ] Get mutable lock on appropriate state store
- [ ] Call the `.create()` or `.add()` method
- [ ] Drop the lock after use
- [ ] Call `record_event()` for audit trail
- [ ] Send `SessionCommand` if needed (e.g., network operations)
- [ ] Return `created_response()` with `record.json()`
- [ ] Add test case to verify the endpoint works
