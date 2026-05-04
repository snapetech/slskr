# Function Signatures and Locations Reference

## JSON Field Extraction Functions

### In main.rs (Lines 2814-2833):

```rust
/// Extract u32 field from JSON body
/// Returns None if field not found or not a valid u32
fn extract_json_u32_field(body: &str, field: &str) -> Option<u32>

/// Extract bool field from JSON body
/// Returns None if field not found or not "true"/"false"
fn extract_json_bool_field(body: &str, field: &str) -> Option<bool>
```

### In utils.rs (utils::):

```rust
/// Extract string field from JSON body, handling escape sequences
/// Returns None if field not found or malformed
pub fn extract_json_string_field(body: &str, field: &str) -> Option<String>

/// Extract array of strings from JSON body
/// Returns None if field not found or malformed
pub fn extract_json_string_array_field(body: &str, field: &str) -> Option<Vec<String>>

/// Find position after a JSON key
/// Used by extract_json_* functions internally
pub fn json_field_after_key<'a>(body: &'a str, key: &str) -> Option<&'a str>
```

---

## Store Creation Methods

### SearchStore::create() - Lines 478-504

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

**Usage:**
```rust
let mut searches = state.searches.write().await;
let record = searches.create(
    "search query".to_string(),
    "peer",           // or "user" 
    Some("target_username".to_string()),
    vec![],
    300               // TTL in seconds
);
drop(searches);
```

---

### TransferQueue::create() - Lines 993-1020

```rust
fn create(
    &mut self,
    direction: u32,            // 0=upload, 1=download
    peer_username: Option<String>,
    filename: String,
    local_path: Option<String>,
    size: Option<u64>,
) -> TransferEntry
```

**Usage:**
```rust
let mut transfers = state.transfers.write().await;
let entry = transfers.create(
    1,                                           // direction
    Some("friend".to_string()),                  // peer_username
    "Music/Song.flac".to_string(),               // filename
    Some("/home/user/downloads/song.flac"),      // local_path
    Some(5000000)                                // size in bytes
);
drop(transfers);
```

---

### MessageStore::add() - Lines 2032-2047

```rust
fn add(
    &mut self,
    username: String,
    direction: &'static str,    // "in" or "out"
    body: String,
) -> MessageRecord
```

**Usage:**
```rust
let mut messages = state.messages.write().await;
let record = messages.add(
    "username".to_string(),
    "in",                           // or "out"
    "Hello there!".to_string()
);
drop(messages);
```

---

### UserStore::watch() - Lines 1472-1497

```rust
fn watch(&mut self, username: String) -> UserRecord
```

**Usage:**
```rust
let mut users = state.users.write().await;
let record = users.watch("username".to_string());
drop(users);
```

---

### UserStore::unwatch() - Lines 1499-1508

```rust
fn unwatch(&mut self, username: &str) -> Option<UserRecord>
```

**Usage:**
```rust
let mut users = state.users.write().await;
if let Some(record) = users.unwatch("username") {
    // User was being watched and is now unwatched
}
drop(users);
```

---

### RoomStore::join() - Lines 2203-2223

```rust
fn join(&mut self, name: String) -> RoomRecord
```

**Usage:**
```rust
let mut rooms = state.rooms.write().await;
let record = rooms.join("Music".to_string());
drop(rooms);
```

---

### RoomStore::leave() - Lines 2225-2232

```rust
fn leave(&mut self, name: &str) -> Option<RoomRecord>
```

**Usage:**
```rust
let mut rooms = state.rooms.write().await;
if let Some(record) = rooms.leave("Music") {
    // Room was joined and is now left
}
drop(rooms);
```

---

### BrowseStore::request() - Lines 1705-1733

```rust
fn request(&mut self, username: String) -> BrowseRecord
```

**Usage:**
```rust
let mut browse = state.browse.write().await;
let record = browse.request("username".to_string());
drop(browse);
```

---

### BrowseStore::next_indirect_token() - Lines 1699-1703

```rust
fn next_indirect_token(&mut self) -> u32
```

**Usage:**
```rust
let mut browse = state.browse.write().await;
let token = browse.next_indirect_token();
drop(browse);
```

---

## Session Command Functions

### send_session_command() - Lines 4755-4761

```rust
async fn send_session_command(
    state: &AppState,
    command: SessionCommand
) -> Result<(), String>
```

**Usage:**
```rust
send_session_command(state, SessionCommand::Connect).await?;

// With data
send_session_command(state, SessionCommand::WatchUser("username".to_string())).await?;

// With struct data
send_session_command(state, SessionCommand::Search {
    token: 1,
    query: "search term".to_string(),
    target: SearchDispatchTarget::Global,
}).await?;
```

---

### try_send_session_command() - Lines 4763-4765

```rust
fn try_send_session_command(state: &AppState, command: SessionCommand)
```

**Usage (non-blocking):**
```rust
try_send_session_command(state, SessionCommand::Disconnect);
```

---

## Response Building Functions

### From routing.rs:

```rust
pub fn created_response(body: String) -> HttpResponse
    // Returns 201 Created

pub fn bad_request_response(message: &str) -> HttpResponse
    // Returns 400 Bad Request with error message

pub fn not_found_response() -> HttpResponse
    // Returns 404 Not Found

pub fn unauthorized_response() -> HttpResponse
    // Returns 401 Unauthorized

pub fn forbidden_response(message: &str) -> HttpResponse
    // Returns 403 Forbidden with error message
```

---

## Utility Functions

### From utils.rs:

```rust
/// Get current UNIX timestamp
pub fn unix_timestamp() -> u64

/// Extract parameters from query string
pub fn query_params(query: &str) -> Vec<(String, String)>
```

### From config.rs:

```rust
/// Escape string for safe JSON inclusion
pub fn json_escape(value: &str) -> String

/// Format Option<u32> for JSON
pub fn json_u32_option(value: Option<u32>) -> String

/// Format Option<u64> for JSON
pub fn json_u64_option(value: Option<u64>) -> String

/// Format Option<usize> for JSON
pub fn json_usize_option(value: Option<usize>) -> String
```

---

## Event Recording

### record_event() - Location: main.rs (search for it)

```rust
async fn record_event(
    state: &AppState,
    event_type: &str,              // e.g., "transfer.created", "user.watched"
    category: &str,                // e.g., "transfers", "users"
    details: Option<String>        // Optional details like "id=123"
)
```

**Usage:**
```rust
record_event(
    state,
    "transfer.created",
    "transfers",
    Some(format!("id={}", entry.id))
).await;
```

---

## AppState Structure - Lines 2361-2374

```rust
struct AppState {
    config: AppConfig,
    session: RwLock<SessionSnapshot>,
    listeners: RwLock<ListenerSnapshot>,
    shares: RwLock<ShareIndexSnapshot>,
    searches: RwLock<SearchStore>,              // <- Lock for searches
    users: RwLock<UserStore>,                   // <- Lock for users
    browse: RwLock<BrowseStore>,                // <- Lock for browse
    messages: RwLock<MessageStore>,             // <- Lock for messages
    rooms: RwLock<RoomStore>,                   // <- Lock for rooms
    transfers: RwLock<TransferQueue>,           // <- Lock for transfers
    events: RwLock<EventStore>,
    session_commands: mpsc::Sender<SessionCommand>, // <- For sending commands
}
```

---

## SessionCommand Variants - Lines 2377-2422

```rust
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
    BrowseFolder { username: String, folder: String },
    IndirectBrowse { username: String, token: u32 },
    RequestUserStats(String),
    TransferPeer { id: u64, username: String },
    IndirectTransfer { id: u64, username: String, token: u32 },
    MessageUser { username: String, body: String },
    MessageAcked { id: u32 },
    RefreshRooms,
    JoinRoom(String),
    LeaveRoom(String),
    SayRoom { room: String, body: String },
}
```

---

## Route Handler Signature

### Main routing function - Lines 2471-2750

```rust
async fn route_http_request_with_headers(
    method: &str,           // "GET", "POST", etc.
    path: &str,             // Full path like "/api/v0/transfers"
    authorization: Option<&str>,  // Auth token if provided
    body: &str,             // Request body (for POST)
    state: &AppState,       // Application state
    headers: RequestSecurityHeaders<'_>,  // Security headers
) -> Result<HttpResponse, String>
```

---

## HTTP Status Codes Returned

```
200 OK           - Successful GET or successful POST action
201 Created      - New resource created
202 Accepted     - Request accepted for async processing
400 Bad Request  - Invalid input/malformed request
401 Unauthorized - No auth provided when required
403 Forbidden    - Auth failed or CSRF check failed
404 Not Found    - Resource not found
409 Conflict     - Business logic constraint violated
```

---

## Quick Copy-Paste: Create extract_json_u64_field()

Add to main.rs after extract_json_u32_field():

```rust
fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}
```

---

## Usage In Route Handler Template

```rust
("POST", "/api/v0/endpoint") => {
    // 1. Extract JSON fields
    let field1 = match extract_json_string_field(body, "field1") {
        Some(f) => f,
        None => return Ok(bad_request_response("field1 required")),
    };
    
    let field2 = extract_json_u64_field(body, "field2").unwrap_or(0);
    
    // 2. Validate
    if field1.is_empty() {
        return Ok(bad_request_response("field1 cannot be empty"));
    }
    
    // 3. Get mutable lock
    let mut store = state.store.write().await;
    
    // 4. Create/modify record
    let record = store.create(field1, field2);
    
    // 5. Drop lock
    drop(store);
    
    // 6. Record event
    record_event(state, "event.type", "category", Some(format!("id={}", record.id))).await;
    
    // 7. Send command if needed
    send_session_command(state, SessionCommand::CommandVariant).await.ok();
    
    // 8. Return response
    Ok(created_response(record.json()))
}
```

