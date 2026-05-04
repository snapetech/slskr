# SoulseekR Code Patterns Analysis

## 1. SearchStore::create() Method

**Location:** Lines 478-504

```rust
fn create(
    &mut self,
    query: String,
    target: &'static str,
    target_name: Option<String>,
    results: Vec<FileEntry>,
    ttl_seconds: u64,
) -> SearchRecord
```

### Parameters:
- `&mut self` - Mutable reference to SearchStore
- `query: String` - Search query string
- `target: &'static str` - Target type ("peer", "user", etc.)
- `target_name: Option<String>` - Optional target name for filtered searches
- `results: Vec<FileEntry>` - Initial search results
- `ttl_seconds: u64` - Time-to-live in seconds for the search

### Returns:
`SearchRecord` - Cloned record that was added to the store

### Implementation Pattern:
```rust
fn create(&mut self, ...) -> SearchRecord {
    let now = unix_timestamp();
    let record = SearchRecord {
        token: self.next_token,
        query,
        target,
        target_name,
        status: "active",
        results: results.iter().map(SearchResultEntry::from_file_entry).collect(),
        expires_at: now.saturating_add(ttl_seconds),
        created_at: now,
        updated_at: now,
    };
    self.next_token = self.next_token.wrapping_add(1).max(1);
    self.records.push(record.clone());
    record
}
```

### Usage in Route Handler:
```rust
let mut searches = state.searches.write().await;
let record = searches.create(
    query,
    "peer",
    None,
    vec![],
    300
);
drop(searches);
```

---

## 2. TransferQueue::create() Method

**Location:** Lines 993-1020

```rust
fn create(
    &mut self,
    direction: u32,
    peer_username: Option<String>,
    filename: String,
    local_path: Option<String>,
    size: Option<u64>,
) -> TransferEntry
```

### Parameters:
- `&mut self` - Mutable reference to TransferQueue
- `direction: u32` - Transfer direction (0=upload, 1=download)
- `peer_username: Option<String>` - Username of peer involved in transfer
- `filename: String` - File name being transferred
- `local_path: Option<String>` - Local file path if applicable
- `size: Option<u64>` - File size in bytes

### Returns:
`TransferEntry` - Cloned entry added to the queue

### Implementation Pattern:
```rust
fn create(&mut self, ...) -> TransferEntry {
    let now = unix_timestamp();
    let token = self.next_token;
    self.next_token = self.next_token.wrapping_add(1).max(1);
    let entry = TransferEntry {
        id: self.next_id,
        direction,
        token,
        peer_username,
        filename,
        local_path,
        size,
        bytes_transferred: 0,
        status: "queued".to_owned(),
        reason: None,
        requested_at: now,
        updated_at: now,
    };
    self.next_id += 1;
    self.push_entry(entry)
}
```

### Usage in Route Handler:
```rust
let mut transfers = state.transfers.write().await;
let entry = transfers.create(
    1,
    Some("username".to_string()),
    "file.mp3".to_string(),
    None,
    Some(5000000)
);
drop(transfers);
```

---

## 3. MessageStore::add() Method

**Location:** Lines 2032-2047

```rust
fn add(
    &mut self,
    username: String,
    direction: &'static str,
    body: String,
) -> MessageRecord
```

### Parameters:
- `&mut self` - Mutable reference to MessageStore
- `username: String` - Username of message sender/recipient
- `direction: &'static str` - Direction ("in" or "out")
- `body: String` - Message content

### Returns:
`MessageRecord` - Cloned record added to the store

### Implementation Pattern:
```rust
fn add(&mut self, username: String, direction: &'static str, body: String) -> MessageRecord {
    let now = unix_timestamp();
    let record = MessageRecord {
        id: self.next_id,
        username,
        direction,
        body,
        acknowledged: false,
        created_at: now,
        updated_at: now,
    };
    self.next_id += 1;
    self.records.push(record.clone());
    self.updated_at = now;
    record
}
```

### Usage in Route Handler:
```rust
let mut messages = state.messages.write().await;
let record = messages.add(
    "username".to_string(),
    "in",
    "Hello there!".to_string()
);
drop(messages);
```

---

## 4. UserStore::watch() Method

**Location:** Lines 1472-1497

```rust
fn watch(&mut self, username: String) -> UserRecord
```

### Parameters:
- `&mut self` - Mutable reference to UserStore
- `username: String` - Username to watch

### Returns:
`UserRecord` - Cloned record added to or updated in the store

### Implementation Pattern:
- Updates existing record if found
- Creates new record if not found
- Both cases return cloned record
- Updates `updated_at` timestamp
- Updates store's `updated_at` timestamp

### Usage in Route Handler:
```rust
let mut users = state.users.write().await;
let record = users.watch("username".to_string());
drop(users);
```

---

## 5. RoomStore::join() Method

**Location:** Lines 2203-2223

```rust
fn join(&mut self, name: String) -> RoomRecord
```

### Parameters:
- `&mut self` - Mutable reference to RoomStore
- `name: String` - Room name to join

### Returns:
`RoomRecord` - Cloned record added to or updated in the store

### Implementation Pattern:
- Updates existing record if found
- Creates new record with default values if not found:
  - `joined: true`
  - `kind: "local"`
  - `user_count: None`
  - `operated: false`
  - `messages: Vec::new()`
- Returns cloned record

### Usage in Route Handler:
```rust
let mut rooms = state.rooms.write().await;
let record = rooms.join("Music".to_string());
drop(rooms);
```

---

## 6. BrowseStore Structure

**Location:** Lines 1684-1733

```rust
struct BrowseStore {
    records: Vec<BrowseRecord>,
    next_indirect_token: u32,
    updated_at: u64,
}
```

### Key Methods:
1. **new()** - Creates new empty store
2. **next_indirect_token()** - Generates unique indirect tokens with wrapping
3. **request(username: String) -> BrowseRecord** - Requests browse of user's shares
   - Updates existing record status to "requested" if found
   - Creates new record if not found
   - Clears folder/entries/indirect_token on new request
   - Returns cloned record

### Implementation Pattern:
```rust
fn request(&mut self, username: String) -> BrowseRecord {
    let now = unix_timestamp();
    if let Some(record) = self.records.iter_mut().find(|record| record.username == username) {
        record.status = "requested";
        record.reason = None;
        record.folder = None;
        record.indirect_token = None;
        record.requested_at = Some(now);
        record.updated_at = now;
        self.updated_at = now;
        return record.clone();
    }
    let record = BrowseRecord {
        username,
        status: "requested",
        entries: Vec::new(),
        reason: None,
        folder: None,
        indirect_token: None,
        requested_at: Some(now),
        updated_at: now,
    };
    self.records.push(record.clone());
    self.updated_at = now;
    record
}
```

---

## 7. JSON Field Extraction Functions

### extract_json_u32_field()

**Location:** Lines 2814-2820

```rust
fn extract_json_u32_field(body: &str, field: &str) -> Option<u32> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}
```

### How to Create extract_json_u64_field()

```rust
fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}
```

### extract_json_bool_field()

**Location:** Lines 2822-2833

```rust
fn extract_json_bool_field(body: &str, field: &str) -> Option<bool> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    let value = after_colon[..end].trim();
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}
```

### Available JSON Extraction Functions in utils.rs

```rust
// From utils.rs
pub fn extract_json_string_field(body: &str, field: &str) -> Option<String>
pub fn extract_json_string_array_field(body: &str, field: &str) -> Option<Vec<String>>
pub fn json_field_after_key<'a>(body: &'a str, key: &str) -> Option<&'a str>

// From config.rs
pub fn json_escape(value: &str) -> String
pub fn json_u32_option(value: Option<u32>) -> String
pub fn json_u64_option(value: Option<u64>) -> String
pub fn json_usize_option(value: Option<usize>) -> String
```

### Usage in Route Handlers:
```rust
// Extract from JSON body
let filename = extract_json_string_field(body, "filename")?;
let size = extract_json_u64_field(body, "size")?;
let direction = extract_json_u32_field(body, "direction")?;
let acknowledged = extract_json_bool_field(body, "acknowledged")?;
```

---

## 8. SessionCommand Enum Definition

**Location:** Lines 2377-2422

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
enum SessionCommand {
    Connect,
    Disconnect,
    Ping,
    CheckPrivileges,
    Search {
        token: u32,
        query: String,
        target: SearchDispatchTarget,
    },
    WatchUser(String),
    UnwatchUser(String),
    BrowseUser(String),
    BrowseFolder {
        username: String,
        folder: String,
    },
    IndirectBrowse {
        username: String,
        token: u32,
    },
    RequestUserStats(String),
    TransferPeer {
        id: u64,
        username: String,
    },
    IndirectTransfer {
        id: u64,
        username: String,
        token: u32,
    },
    MessageUser {
        username: String,
        body: String,
    },
    MessageAcked {
        id: u32,
    },
    RefreshRooms,
    JoinRoom(String),
    LeaveRoom(String),
    SayRoom {
        room: String,
        body: String,
    },
}
```

---

## 9. Sending SessionCommand - Pattern

**Location:** Lines 4755-4765

```rust
async fn send_session_command(state: &AppState, command: SessionCommand) -> Result<(), String> {
    state
        .session_commands
        .send(command)
        .await
        .map_err(|_| "session manager is not running".to_owned())
}

fn try_send_session_command(state: &AppState, command: SessionCommand) {
    let _ = state.session_commands.try_send(command);
}
```

### Important Points:
1. **Async variant** (`send()`) - Awaits the operation, returns Result
   - Use when you need to know if command was sent successfully
   - Should NOT be used in non-async context
   
2. **Non-async variant** (`try_send()`) - Non-blocking, ignores errors
   - Use for fire-and-forget operations
   - Doesn't return Result

### Usage in Route Handlers:
```rust
// When you want to await the send operation
async fn some_route_handler(state: &AppState, ...) -> Result<HttpResponse, String> {
    send_session_command(state, SessionCommand::Connect)
        .await?;
    
    // Or for commands with data
    send_session_command(state, SessionCommand::Search {
        token: 1,
        query: "search term".to_string(),
        target: SearchDispatchTarget::Global,
    })
    .await?;
    
    Ok(...)
}

// Or non-blocking
fn some_function(state: &AppState) {
    try_send_session_command(state, SessionCommand::Disconnect);
}
```

### From AppState:
```rust
struct AppState {
    ...
    session_commands: mpsc::Sender<SessionCommand>,
}
```

---

## 10. Route Handler Pattern

**Location:** Lines 2796-2812 and 2471-2750

### Function Signature:
```rust
async fn route_http_request(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
) -> Result<HttpResponse, String>
```

### Current Structure:
```rust
async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: RequestSecurityHeaders<'_>,
) -> Result<HttpResponse, String> {
    let route = routing::parse_route(method, path);
    
    // Auth check
    if let Err(_) = routing::check_route_auth(...) {
        return Ok(if authorization.is_none() {
            routing::unauthorized_response()
        } else {
            routing::forbidden_response("unauthorized")
        });
    }
    
    // Route matching with pattern
    match (method, route.normalized_path) {
        ("GET", "/") => Ok(index_html_response()),
        ("GET", "/api/health") => Ok(health_response()),
        ("POST", "/api/shares/rescan") => { /* handler */ },
        ("GET", path) if search_token_path(route.normalized_path, "").is_some() => {
            // Guard clause for dynamic paths
        },
        _ => Ok(routing::not_found_response()),
    }
}
```

### Response Types:
```rust
pub fn created_response(body: String) -> HttpResponse {
    HttpResponse {
        status: "201 Created",
        content_type: "application/json",
        body,
    }
}

pub fn bad_request_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", json_escape(message)),
    }
}

pub fn conflict_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "409 Conflict",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", json_escape(message)),
    }
}
```

---

## 11. Complete Route Handler Example

### From test at line 7404:

```rust
// Test calling POST /api/v0/transfers
let body = r#"{"filename":"Remote/One.flac","size":10}"#;
let created = route_http_request("POST", "/api/v0/transfers", None, body, &state)
    .await
    .expect("create transfer");
assert_eq!(created.status, "201 Created");
```

### Pattern for Creating a New POST Handler:

```rust
("POST", "/api/v0/transfers") => {
    // Extract JSON fields
    let filename = match extract_json_string_field(body, "filename") {
        Some(f) => f,
        None => return Ok(bad_request_response("filename required")),
    };
    
    let direction = extract_json_u32_field(body, "direction").unwrap_or(1);
    let size = extract_json_u64_field(body, "size");
    let peer_username = extract_json_string_field(body, "peer_username");
    let local_path = extract_json_string_field(body, "local_path");
    
    // Validate
    if filename.is_empty() {
        return Ok(bad_request_response("filename cannot be empty"));
    }
    
    // Create entry
    let mut transfers = state.transfers.write().await;
    let entry = transfers.create(direction, peer_username, filename, local_path, size);
    drop(transfers);
    
    // Record event
    record_event(state, "transfer.created", "transfers", Some(format!("id={}", entry.id))).await;
    
    // Return response
    Ok(created_response(entry.json()))
}
```

---

## Summary of Key Patterns

1. **Write Lock Pattern**: `let mut store = state.store.write().await;` → modify → `drop(store)`
2. **Read Lock Pattern**: `let store = state.read().await;` → read → `drop(store)` (or auto-drop)
3. **Token Generation**: Stores use `wrapping_add(1).max(1)` to increment tokens, ensuring they never wrap to 0
4. **Timestamps**: Always use `unix_timestamp()` for current time
5. **Record Creation**: All stores return `Record.clone()` after adding to internal vector
6. **JSON Extraction**: Use helper functions like `extract_json_string_field()`, `extract_json_u32_field()`, etc.
7. **Error Handling**: Use `Option` return and convert with `.ok()` or pattern matching
8. **Response Status Codes**: 
   - 201 Created - for POST that creates resources
   - 200 OK - for successful GET/POST operations
   - 400 Bad Request - for invalid input
   - 409 Conflict - for constraint violations
   - 404 Not Found - for missing resources
9. **Session Commands**: Use `send_session_command()` (async) or `try_send_session_command()` (fire-and-forget)
10. **Event Recording**: Call `record_event()` after operations for audit trail
