# slskr Database & Storage Layer Analysis

## Executive Summary

The slskr codebase currently has **NO persistent database layer**. All data (searches, transfers, messages, users, rooms) is stored in-memory using Rust `HashMap` and `Vec` structures. There is a placeholder `persistence.rs` module with in-memory structures but no actual SQLite or database integration.

The only persistence implemented is:
- **Share index cache** (TSV format): `shares.tsv`
- **Transfer events log** (TSV format): `transfer.events.tsv`
- **Transfer state snapshot** (JSON format): `transfer.state.json`

---

## Current Storage Implementation

### 1. File-Based Storage Locations

**State Directory:**
- Environment: `$SLSKR_STATE_DIR` (default: `~/.local/state/slskr`)
- Configured in: `/crates/slskr/src/config.rs` lines 58-59
- Default calculation: Uses `XDG_STATE_HOME` or `$HOME/.local/state`

**Files Created:**
```
~/.local/state/slskr/
├── share-index.tsv          # Share catalog cache
├── transfer.events.tsv      # Transfer event log (append-only)
└── transfer.state.json      # Transfer queue snapshot
```

### 2. Storage Module (`/crates/slskr/src/storage.rs`)

**Current Functionality:**
- File entry serialization/deserialization
- Share indexing and caching (TSV format)
- Transfer state management (JSON)
- Folder navigation and grouping
- Payload compression/decompression (zlib)

**Key Functions:**
- `share_cache_path()` - Returns path to `shares.tsv`
- `write_share_cache()` - Writes FileEntry array as TSV
- `transfer_events_path()` - Returns path to `transfer.events.tsv`
- `transfer_state_path()` - Returns path to `transfer.state.json`
- `append_transfer_event()` - Appends transfer event to TSV
- `build_shared_file_list_payload()` - Encodes shares for protocol
- `parse_shared_file_list_payload()` - Decodes share payloads

**Format Details:**
- **Share Cache**: Tab-separated values with header, escaped special chars
- **Transfer Events**: TSV with columns: `timestamp`, `status`, `bytes_transferred`, `filename`
- **Transfer State**: JSON format with version number and array of TransferEntry objects

### 3. Persistence Module (`/crates/slskr/src/persistence.rs`)

**Current State:** Placeholder implementation

**Purpose:** Listed as future SQLite backend location (lines 3-4):
```rust
/// Production implementation would use sqlx with SQLite backend.
/// In production, replace with sqlx + SQLite backend.
```

**Data Structures Defined:**
```rust
pub struct SearchRecord {
    pub id: String,
    pub query: String,
    pub status: String,
    pub result_count: u32,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub room: Option<String>,
    pub target: Option<String>,
}

pub struct TransferRecord {
    pub id: String,
    pub direction: String,
    pub filename: String,
    pub peer_username: String,
    pub filesize: u64,
    pub progress: u64,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

pub struct MessageRecord {
    pub id: String,
    pub username: String,
    pub content: String,
    pub direction: String,
    pub read: bool,
    pub created_at: i64,
}
```

**In-Memory Database Manager:**
- Placeholder using `HashMap<String, Record>` for searches, transfers, messages
- Methods for CRUD operations all return `Ok()` without persistence
- Includes test suite but no actual database backend

---

## In-Memory Data Structures (main.rs)

All runtime data is managed through the `AppState` struct with `RwLock` wrappers for thread-safe access:

### AppState Structure
```rust
struct AppState {
    config: AppConfig,
    session: RwLock<SessionSnapshot>,           // Server connection state
    listeners: RwLock<ListenerSnapshot>,        // Active listeners
    shares: RwLock<ShareIndexSnapshot>,         // Shared files index
    searches: RwLock<SearchStore>,              // Active & historical searches
    users: RwLock<UserStore>,                   // Watched/tracked users
    browse: RwLock<BrowseStore>,                // Browse session cache
    messages: RwLock<MessageStore>,             // Private messages
    rooms: RwLock<RoomStore>,                   // Joined chat rooms
    transfers: RwLock<TransferQueue>,           // Active & historical transfers
    events: RwLock<EventStore>,                 // General event log
    session_commands: mpsc::Sender<SessionCommand>,
}
```

### Data Stores

**SearchStore:**
- Vector of `SearchRecord` entries
- Tracks token counter for new searches
- Lost on server restart

**TransferQueue:**
- Vector of `TransferEntry` structures
- Limited by `transfer_history_limit` config (default: 500)
- Only partial persistence: recent transfers saved to `transfer.state.json` on update
- Transfer events logged to `transfer.events.tsv` but not fully indexed

**MessageStore:**
- Vector of `MessageRecord` entries
- Tracks message ID counter
- Never persisted - lost on restart

**UserStore:**
- Vector of `UserRecord` entries (watched users, stats)
- Contains: username, status, speed, upload count, file/directory counts
- Never persisted - lost on restart

**BrowseStore:**
- Vector of `BrowseRecord` entries
- Caches user file browse sessions
- Never persisted

**RoomStore:**
- Vector of `RoomRecord` entries (joined chat rooms)
- Never persisted - needs to be rejoined after restart

**EventStore:**
- Vector of `EventRecord` entries
- In-memory event log with configurable history limit
- Never persisted - lost on restart

**ShareIndexSnapshot:**
- Indexed cache of shared files
- Loaded from `shares.tsv` on startup
- Updated when rescan is triggered
- Entry structure: `FileEntry { code, filename, size, extension, attributes }`

---

## Data Requiring Persistence (NOT CURRENTLY PERSISTED)

### Critical (User-Facing Data)

1. **Search History**
   - Query text and parameters
   - Result count and status
   - Timestamps
   - API: `GET /api/searches`, `POST /api/searches`

2. **Transfer History**
   - **PARTIAL**: Last 500 transfers saved to JSON snapshot
   - **MISSING**: Full transfer archive, historical analytics
   - Includes: direction, peer, filename, bytes transferred, status
   - API: `GET /api/transfers`, `GET /api/transfers/stats`

3. **Message Archive**
   - **NONE**: All messages lost on restart
   - Includes: username, direction (sent/received), content, read status
   - API: `GET /api/messages`, `POST /api/messages`

4. **User Watch List & Stats**
   - **NONE**: Watched user list lost on restart
   - Includes: username, status, upload count, file count
   - API: `GET /api/users`, `POST /api/users/watch`

5. **Room Membership**
   - **NONE**: Joined rooms list lost on restart
   - Need to rejoin all rooms after restart
   - API: `POST /api/rooms/refresh`, `POST /api/rooms/:name/join`

6. **User Statistics**
   - Session-wide stats about user activity
   - Never persisted between restarts
   - API: `GET /api/stats`

### Infrastructure Data

7. **Share Index** (CURRENTLY PERSISTED)
   - ✅ Cached in `shares.tsv` (tab-separated format)
   - Reloaded on server start
   - Updated on rescan

8. **Configuration** (FILE-BASED, NOT DATABASE)
   - Stored in TOML config files or environment variables
   - Not managed by database layer

---

## API Endpoints & Their Persistence Requirements

### Currently Working with In-Memory Data
```
GET  /api/searches                    # Return in-memory search history
GET  /api/searches/:token             # Single search details
POST /api/searches                    # Create new search
POST /api/searches/:token/complete    # Mark search complete
POST /api/searches/prune              # Remove old searches

GET  /api/transfers                   # Return transfer queue (from memory)
GET  /api/transfers/stats             # Transfer statistics
POST /api/transfers                   # Create new transfer

GET  /api/messages                    # Return messages (from memory)
POST /api/messages                    # Send message
POST /api/messages/inbound            # Receive message

GET  /api/users                       # Return watched users (from memory)
POST /api/users/watch                 # Add watch
DELETE /api/users/watch/:username     # Remove watch

GET  /api/rooms                       # Return joined rooms (from memory)
POST /api/rooms/:name/join            # Join room
DELETE /api/rooms/:name/join          # Leave room

GET  /api/events                      # Return event log (in-memory, limited size)
GET  /api/session                     # Server connection state

GET  /api/shares                      # Share index (loaded from shares.tsv)
POST /api/shares/rescan               # Update shares.tsv
```

### GraphQL Queries (Placeholder Implementation)
Located in `/crates/slskr/src/graphql.rs` - Currently returns mock data:
- `searches(limit, offset)`
- `search(id)`
- `transfers(direction, limit, offset)`
- `transfer(id)`
- `messages(username, limit, offset)`
- `message(id)`
- `users(limit, offset)`
- `user(username)`
- `stats()`

---

## Configuration & Dependencies

### Cargo.toml Analysis
**Current Dependencies (slskr crate):**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
slskr-client = { version = "0.0.0", path = "../slskr-client" }
slskr-cli = { version = "0.0.0", path = "../slskr-cli" }
tokio = { version = "1", features = [...] }
toml = "0.8"
```

**Missing:**
- ❌ `sqlx` - No SQL query builder
- ❌ `sqlite` / `rusqlite` - No SQLite driver
- ❌ `tokio-sqlite` - No async SQLite
- ❌ `diesel`, `sea-orm` - No ORM
- ❌ `sqlalchemy` - Not applicable (Rust)

**Implications:**
- No database layer in dependencies
- Would need to add SQLite dependency (e.g., `sqlx` with `runtime-tokio-native-tls`)
- No migration framework (would need `sqlx-cli` or custom migrations)

---

## Limitations of Current Approach

### Data Loss
1. All searches disappear on restart
2. All messages lost (not even event log)
3. All room subscriptions lost
4. User watch list lost
5. Only last 500 transfers saved as snapshot
6. Transfer events logged but not queryable

### Performance Issues
1. All operations O(n) - linear scan of in-memory vectors
2. No indexing on common queries (by username, status, date)
3. No pagination efficiency
4. No aggregation optimizations

### Scalability Limitations
1. All data loaded into RAM on startup
2. No sharding or partitioning
3. Share index could become huge (capped at `share_scan_max_files` = 50,000)
4. MessageStore grows unbounded during session

### Query Limitations
1. No SQL filtering/sorting/aggregation
2. GraphQL resolvers return mock data
3. Can't query historical analytics
4. No full-text search capability

---

## Required Data Persistence Implementation

### Immediate Priorities

**Tier 1 (High Impact):**
1. **Search History** - Critical for UX, currently 100% lost
2. **Message Archive** - User-facing data, currently 100% lost
3. **Transfer Archive** - Currently limited to recent 500
4. **User Stats** - Session stats should survive restarts

**Tier 2 (Medium Impact):**
1. **Room History** - Track joined rooms/subscriptions
2. **User Watch List** - Preserve watched user list
3. **Event Log** - Persistent audit trail
4. **Browse Cache** - Optimize repeated browses

**Tier 3 (Infrastructure):**
1. **API Token Usage** - Rate limiting history
2. **Error Logs** - Diagnostic persistence
3. **Configuration Snapshots** - Track config changes

---

## Recommended Database Schema (Not Yet Implemented)

### Core Tables
```sql
-- Search History
CREATE TABLE searches (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    status TEXT NOT NULL,
    result_count INTEGER,
    room_name TEXT,
    target_username TEXT,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    created_by_user TEXT
);

-- Transfer History
CREATE TABLE transfers (
    id TEXT PRIMARY KEY,
    direction TEXT NOT NULL,  -- 'upload' or 'download'
    filename TEXT NOT NULL,
    peer_username TEXT NOT NULL,
    filesize INTEGER,
    bytes_transferred INTEGER,
    local_path TEXT,
    status TEXT NOT NULL,
    reason TEXT,
    requested_at INTEGER NOT NULL,
    completed_at INTEGER
);

-- Message Archive
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    direction TEXT NOT NULL,  -- 'inbound' or 'outbound'
    content TEXT NOT NULL,
    read_at INTEGER,
    created_at INTEGER NOT NULL
);

-- User Tracking
CREATE TABLE users (
    username TEXT PRIMARY KEY,
    status TEXT,
    average_speed INTEGER,
    upload_count INTEGER,
    file_count INTEGER,
    directory_count INTEGER,
    watched BOOLEAN,
    last_seen INTEGER
);

-- Room Subscriptions
CREATE TABLE rooms (
    name TEXT PRIMARY KEY,
    joined_at INTEGER NOT NULL,
    left_at INTEGER
);

-- Events Log
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    details TEXT,
    created_at INTEGER NOT NULL
);
```

### Indices
```sql
CREATE INDEX idx_searches_created ON searches(created_at DESC);
CREATE INDEX idx_transfers_created ON transfers(requested_at DESC);
CREATE INDEX idx_transfers_status ON transfers(status);
CREATE INDEX idx_messages_username ON messages(username);
CREATE INDEX idx_messages_created ON messages(created_at DESC);
CREATE INDEX idx_events_created ON events(created_at DESC);
```

---

## File Locations Summary

### Source Files
- **Main persistence placeholder**: `/crates/slskr/src/persistence.rs`
- **File-based storage**: `/crates/slskr/src/storage.rs`
- **Configuration**: `/crates/slskr/src/config.rs`
- **Main application state**: `/crates/slskr/src/main.rs` (10,351 lines)
- **GraphQL schema**: `/crates/slskr/src/graphql.rs` (344 lines)

### Runtime State Files (Created by App)
- **Shares index**: `{state_dir}/share-index.tsv`
- **Transfer events**: `{state_dir}/transfer.events.tsv`
- **Transfer state**: `{state_dir}/transfer.state.json`

### Configuration Files (User-Provided)
- **Config file**: Specified via `SLSKR_CONFIG` env var or auto-detected
- **State directory**: `$SLSKR_STATE_DIR` (default: `~/.local/state/slskr`)

---

## Conclusion

The slskr codebase currently implements **file-based persistence only for share index and transfer snapshots**. All other data is purely in-memory and lost on restart. 

To achieve production-quality persistence:
1. Add SQLite dependency (`sqlx`)
2. Implement database initialization and schema creation
3. Replace in-memory `HashMap` structures with database queries
4. Create migration system for schema versioning
5. Implement async database operations within tokio runtime
6. Add persistence for searches, messages, users, rooms, and events

The placeholder `persistence.rs` module shows the intended API structure but contains no actual implementation.

