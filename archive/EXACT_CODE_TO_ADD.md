# Exact Code to Add - Ready to Copy-Paste

## 1. Add extract_json_u64_field() Function

**Location:** In main.rs, after line 2833 (after `extract_json_bool_field`)

**Code to add:**
```rust
fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}
```

**Why:** TransferQueue::create() takes size as Option<u64>. This function extracts u64 fields from JSON.

---

## 2. Add conflict_response() Function (Optional but Recommended)

**Location:** In routing.rs, after the other response builder functions

**Code to add:**
```rust
pub fn conflict_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "409 Conflict",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}
```

**Why:** Some endpoints (like start transfer when limit reached) need to return 409 Conflict.

---

## 3. Path Extraction Helper Functions

**Location:** In main.rs, before the route_http_request_with_headers function (around line 2470)

**Code to add:**
```rust
// Path extraction helpers for dynamic routes
fn transfer_start_path(normalized_path: &str) -> Option<u64> {
    if normalized_path.starts_with("/api/transfers/") && normalized_path.ends_with("/start") {
        let middle = normalized_path.strip_prefix("/api/transfers/")?
            .strip_suffix("/start")?;
        middle.parse().ok()
    } else {
        None
    }
}

fn user_watch_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/users/") && normalized_path.ends_with("/watch") {
        normalized_path.strip_prefix("/api/users/")?
            .strip_suffix("/watch")
    } else {
        None
    }
}

fn browse_user_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/browse/") {
        normalized_path.strip_prefix("/api/browse/")
    } else {
        None
    }
}

fn room_join_path(normalized_path: &str) -> Option<&str> {
    if normalized_path.starts_with("/api/rooms/") && normalized_path.ends_with("/join") {
        normalized_path.strip_prefix("/api/rooms/")?
            .strip_suffix("/join")
    } else {
        None
    }
}
```

**Why:** These extract user-provided path parameters from normalized paths (e.g., extract "123" from "/api/v0/transfers/123/start").

---

## 4. Add POST Routes to route_http_request_with_headers()

**Location:** In main.rs, in the match statement at line 2489+, before the `_ => Ok(routing::not_found_response()),` line

**Code to add (one at a time as you implement them):**

### 4a. POST /api/v0/transfers

```rust
("POST", "/api/v0/transfers") => {
    let filename = match extract_json_string_field(body, "filename") {
        Some(f) => f,
        None => return Ok(routing::bad_request_response("filename required")),
    };
    
    let direction = extract_json_u32_field(body, "direction").unwrap_or(1);
    let size = extract_json_u64_field(body, "size");
    let peer_username = extract_json_string_field(body, "peer_username");
    let local_path = extract_json_string_field(body, "local_path");
    
    let mut transfers = state.transfers.write().await;
    let entry = transfers.create(direction, peer_username, filename, local_path, size);
    drop(transfers);
    
    record_event(state, "transfer.created", "transfers", Some(format!("id={}", entry.id))).await;
    
    Ok(routing::created_response(entry.json()))
}
```

### 4b. POST /api/v0/messages

```rust
("POST", "/api/v0/messages") => {
    let username = match extract_json_string_field(body, "username") {
        Some(u) => u,
        None => return Ok(routing::bad_request_response("username required")),
    };
    
    let body_text = match extract_json_string_field(body, "body") {
        Some(b) => b,
        None => return Ok(routing::bad_request_response("body required")),
    };
    
    let mut messages = state.messages.write().await;
    let record = messages.add(username, "in", body_text);
    drop(messages);
    
    record_event(state, "message.created", "messages", Some(format!("id={}", record.id))).await;
    
    Ok(routing::created_response(record.json()))
}
```

### 4c. POST /api/v0/users/{username}/watch

```rust
("POST", path) if user_watch_path(route.normalized_path).is_some() => {
    let username = user_watch_path(route.normalized_path).unwrap().to_string();
    
    let mut users = state.users.write().await;
    let record = users.watch(username.clone());
    drop(users);
    
    record_event(state, "user.watched", "users", Some(username.clone())).await;
    send_session_command(state, SessionCommand::WatchUser(username)).await.ok();
    
    Ok(routing::created_response(record.json()))
}
```

### 4d. POST /api/v0/browse/{username}

```rust
("POST", path) if browse_user_path(route.normalized_path).is_some() => {
    let username = browse_user_path(route.normalized_path).unwrap().to_string();
    
    let mut browse = state.browse.write().await;
    let record = browse.request(username.clone());
    drop(browse);
    
    record_event(state, "browse.requested", "browse", Some(username.clone())).await;
    send_session_command(state, SessionCommand::BrowseUser(username)).await.ok();
    
    Ok(routing::created_response(record.json()))
}
```

### 4e. POST /api/v0/rooms/{name}/join

```rust
("POST", path) if room_join_path(route.normalized_path).is_some() => {
    let room_name = room_join_path(route.normalized_path).unwrap().to_string();
    
    let mut rooms = state.rooms.write().await;
    let record = rooms.join(room_name.clone());
    drop(rooms);
    
    record_event(state, "room.joined", "rooms", Some(room_name.clone())).await;
    send_session_command(state, SessionCommand::JoinRoom(room_name)).await.ok();
    
    Ok(routing::created_response(record.json()))
}
```

### 4f. POST /api/v0/transfers/{id}/start

```rust
("POST", path) if transfer_start_path(route.normalized_path).is_some() => {
    let id = transfer_start_path(route.normalized_path).unwrap();
    
    let mut transfers = state.transfers.write().await;
    if let Some(entry) = transfers.get_mut(id) {
        if entry.status == "queued" {
            entry.status = "in_progress".to_owned();
            entry.updated_at = unix_timestamp();
            let json_response = entry.json();
            drop(transfers);
            
            record_event(state, "transfer.started", "transfers", Some(format!("id={}", entry.id))).await;
            
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: json_response,
            })
        } else {
            drop(transfers);
            Ok(routing::conflict_response("transfer is not queued"))
        }
    } else {
        drop(transfers);
        Ok(routing::not_found_response())
    }
}
```

---

## Summary of Required Changes

1. **Add 1 function:** `extract_json_u64_field()` in main.rs (after line 2833)
2. **Add 1 function (optional):** `conflict_response()` in routing.rs
3. **Add 4 helper functions:** Path extraction functions in main.rs (before route handler)
4. **Add 6 route handlers:** In the match statement in `route_http_request_with_headers()` (before the `_ => Ok(routing::not_found_response())` line)

Total: ~150 lines of new code across 2 files (main.rs and routing.rs)

---

## Integration Order

1. First, add `extract_json_u64_field()` function
2. Then, add path extraction helper functions
3. Then, add route handlers one at a time (test each one as you add it)
4. Finally, add `conflict_response()` if needed

This order ensures each piece builds on the previous one and is testable independently.

