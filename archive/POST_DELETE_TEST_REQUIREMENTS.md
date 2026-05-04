# POST/DELETE Endpoint Test Requirements - Comprehensive List

## SUMMARY
Found 22 POST/DELETE test endpoints that require implementation. These endpoints are tested in `/home/keith/Documents/code/soulseekR/crates/slskr/src/main.rs` and currently NOT routed in the main routing match statement (lines 2489-2750).

---

# 1. SEARCH ENDPOINTS (4 endpoints)

## 1.1 POST /api/v0/searches - Create Search
- **HTTP Method & Path:** POST /api/v0/searches
- **Request Body JSON:** `{"query":"test flac","target":"global","username":"friend","room":"music","ttl_seconds":300}`
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "token" (numeric)
  - Body contains "query" field
  - Body contains "target" field
  - Body contains "status": "active"
  - Body contains "result_count"
- **SessionCommand Sent:** SessionCommand::Search { token: u32, query: String, target: SearchDispatchTarget }
  - SearchDispatchTarget variants: Global, User(String), Room(String), Wishlist
- **Validation Requirements:**
  - "query" field is required (400 Bad Request if missing)
  - "username" required if target="user" (400 Bad Request)
  - Returns 201 Created on success

## 1.2 POST /api/v0/searches/{token}/complete - Complete Search
- **HTTP Method & Path:** POST /api/v0/searches/{token}/complete (e.g., /api/v0/searches/1/complete)
- **Request Body JSON:** (empty or `{}`)
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "status": "completed"
- **SessionCommand Sent:** None
- **Validation Requirements:**
  - Token must reference existing search

## 1.3 POST /api/v0/searches/prune - Prune Expired Searches
- **HTTP Method & Path:** POST /api/v0/searches/prune
- **Request Body JSON:** (empty)
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "pruned" field (numeric)
  - Body contains "remaining" field (numeric)
- **SessionCommand Sent:** None

## 1.4 POST /api/v0/search-responses - Record Search Response
- **HTTP Method & Path:** POST /api/v0/search-responses
- **Request Body JSON (Full):** `{"token":1,"peer_username":"peer1","filename":"Remote/Song.mp3","size":99,"slot_free":false,"average_speed":12,"queue_length":3}`
- **Request Body JSON (Flattened):** `{"token":1,"peer_username":"peer1","filename":"Remote/Song.mp3","size":99}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "result_count"
  - Body contains "peer_username"
  - Body contains "filename"
  - Body contains "extension" (extracted from filename)
  - Body contains "slot_free" (boolean)
  - Body contains "average_speed" (numeric)
  - Body contains "queue_length" (numeric)
- **SessionCommand Sent:** None
- **Validation Requirements:**
  - "token" field is required (400 Bad Request if missing)

---

# 2. TRANSFER ENDPOINTS (4 endpoints)

## 2.1 POST /api/v0/transfers - Create Transfer
- **HTTP Method & Path:** POST /api/v0/transfers
- **Request Body JSON:** `{"direction":1,"filename":"Remote/Song.flac","peer_username":"friend","local_path":"/tmp/file.flac","size":100}`
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "id" field (numeric)
  - Body contains "status": "queued"
  - Body contains "token" field when peer_username is present
- **SessionCommand Sent:** None
- **Validation Requirements:**
  - "filename" field is required (400 Bad Request if missing)
  - "direction" defaults to 0 if not specified
  - "local_path" optional but triggers local file operations

## 2.2 POST /api/v0/transfers/{id}/start - Start Transfer
- **HTTP Method & Path:** POST /api/v0/transfers/{id}/start (e.g., /api/v0/transfers/1/start)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 200 OK (or 409 Conflict on validation failure)
- **Key Assertions:**
  - Body contains "status" field (possible values: in_progress, succeeded, failed, peer_lookup, rejected)
  - Body contains "bytes_transferred" field
  - For peer transfers: Body contains "status": "peer_lookup"
- **SessionCommand Sent:** SessionCommand::TransferPeer { id: u64, username: String } (only for peer transfers)
- **Validation Requirements:**
  - Returns 409 Conflict with error "transfer limit reached" if max active transfers policy violated
  - Returns 409 Conflict with error "outbound transfers are disabled" if peer transfer and outbound disabled
  - Auto-executes local file operations if local_path is present
  - Sets status to "succeeded" or "failed" based on local file read

## 2.3 POST /api/v0/transfers/{id}/progress - Update Progress
- **HTTP Method & Path:** POST /api/v0/transfers/{id}/progress (e.g., /api/v0/transfers/1/progress)
- **Request Body JSON:** `{"bytes_transferred":40}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "bytes_transferred" field with updated value (40)
- **SessionCommand Sent:** None

## 2.4 POST /api/v0/transfers/{id}/complete - Complete Transfer
- **HTTP Method & Path:** POST /api/v0/transfers/{id}/complete (e.g., /api/v0/transfers/1/complete)
- **Request Body JSON:** `{"bytes_transferred":100,"status":"succeeded"}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "status" field
  - Body contains "bytes_transferred" field
- **SessionCommand Sent:** None

---

# 3. MESSAGE ENDPOINTS (3 endpoints)

## 3.1 POST /api/v0/messages - Send Outbound Message
- **HTTP Method & Path:** POST /api/v0/messages
- **Request Body JSON:** `{"username":"friend","body":"hello"}`
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "direction": "outbound"
  - Body contains "acknowledged": false
  - Body contains "username" field
  - Body contains "body" field
  - Body contains "id" field (numeric)
- **SessionCommand Sent:** SessionCommand::MessageUser { username: String, body: String }

## 3.2 POST /api/v0/messages/inbound - Record Inbound Message
- **HTTP Method & Path:** POST /api/v0/messages/inbound
- **Request Body JSON:** `{"username":"friend","body":"hi"}`
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "direction": "inbound"
  - Body contains "username" field
  - Body contains "body" field
  - Body contains "id" field (numeric)
- **SessionCommand Sent:** None

## 3.3 POST /api/v0/messages/{id}/ack - Acknowledge Message
- **HTTP Method & Path:** POST /api/v0/messages/{id}/ack (e.g., /api/v0/messages/1/ack)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "acknowledged": true
- **SessionCommand Sent:** SessionCommand::MessageAcked { id: u64 }

---

# 4. ROOM ENDPOINTS (4 endpoints)

## 4.1 POST /api/v0/rooms/refresh - Refresh Room List
- **HTTP Method & Path:** POST /api/v0/rooms/refresh
- **Request Body JSON:** (empty)
- **Expected Response Status:** 202 Accepted
- **Key Assertions:**
  - Response status is 202 (no specific body assertions in tests)
- **SessionCommand Sent:** SessionCommand::RefreshRooms
- **Validation Requirements:**
  - Enqueues RefreshRooms command

## 4.2 POST /api/v0/rooms/{name}/join - Join Room
- **HTTP Method & Path:** POST /api/v0/rooms/{name}/join (e.g., /api/v0/rooms/music/join)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "name" field matching the room name
  - Body contains "joined": true
- **SessionCommand Sent:** SessionCommand::JoinRoom(String) where String is room name
- **Validation Requirements:**
  - Creates room record or updates existing record

## 4.3 DELETE /api/v0/rooms/{name}/join - Leave Room
- **HTTP Method & Path:** DELETE /api/v0/rooms/{name}/join (e.g., DELETE /api/v0/rooms/music/join)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "joined": false
  - Body contains "name" field
- **SessionCommand Sent:** SessionCommand::LeaveRoom(String) where String is room name

## 4.4 POST /api/v0/rooms/{name}/messages - Record Room Message
- **HTTP Method & Path:** POST /api/v0/rooms/{name}/messages (e.g., /api/v0/rooms/music/messages)
- **Request Body JSON:** `{"username":"friend","body":"track?"}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "message_count" field (numeric)
  - Body contains "body" field matching input
  - Body contains "username" field
- **SessionCommand Sent:** SessionCommand::SayRoom { room: String, body: String }

---

# 5. USER ENDPOINTS (6 endpoints)

## 5.1 POST /api/v0/users/watch - Watch User
- **HTTP Method & Path:** POST /api/v0/users/watch
- **Request Body JSON:** `{"username":"friend"}`
- **Expected Response Status:** 201 Created
- **Key Assertions:**
  - Body contains "username" field
  - Body contains "watched": true
- **SessionCommand Sent:** SessionCommand::WatchUser(String) where String is username
- **Validation Requirements:**
  - "username" field is required (400 Bad Request if missing)

## 5.2 DELETE /api/v0/users/{username}/watch - Unwatch User
- **HTTP Method & Path:** DELETE /api/v0/users/{username}/watch (e.g., /api/v0/users/friend/watch)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "watched": false
  - Body contains "username" field
- **SessionCommand Sent:** SessionCommand::UnwatchUser(String) where String is username

## 5.3 POST /api/v0/users/{username}/stats/request - Request User Stats
- **HTTP Method & Path:** POST /api/v0/users/{username}/stats/request (e.g., /api/v0/users/friend/stats/request)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 202 Accepted
- **Key Assertions:**
  - Response status is 202 (no specific body assertions in tests)
- **SessionCommand Sent:** SessionCommand::RequestUserStats(String) where String is username

## 5.4 POST /api/v0/users/{username}/browse/request - Request Browse
- **HTTP Method & Path:** POST /api/v0/users/{username}/browse/request (e.g., /api/v0/users/friend/browse/request)
- **Request Body JSON:** (empty)
- **Expected Response Status:** 202 Accepted
- **Key Assertions:**
  - Body contains "status": "requested"
- **SessionCommand Sent:** SessionCommand::BrowseUser(String) where String is username

## 5.5 POST /api/v0/users/{username}/browse/folder - Request Folder Browse
- **HTTP Method & Path:** POST /api/v0/users/{username}/browse/folder (e.g., /api/v0/users/friend/browse/folder)
- **Request Body JSON:** `{"folder":"Remote/Album"}`
- **Expected Response Status:** 202 Accepted
- **Key Assertions:**
  - Body contains "status": "requested"
  - Body contains "folder" field matching input
- **SessionCommand Sent:** SessionCommand::BrowseFolder { username: String, folder: String }

## 5.6 POST /api/v0/users/{username}/browse/fail - Mark Browse Failed
- **HTTP Method & Path:** POST /api/v0/users/{username}/browse/fail (e.g., /api/v0/users/friend/browse/fail)
- **Request Body JSON:** `{"reason":"peer timed out"}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "status": "failed"
  - Body contains "reason" field matching input
- **SessionCommand Sent:** None

---

# 6. BROWSE-RESPONSE ENDPOINTS (1 endpoint)

## 6.1 POST /api/v0/browse-responses - Record Browse Response
- **HTTP Method & Path:** POST /api/v0/browse-responses
- **Request Body JSON (Array Format):** `{"username":"friend","complete":false,"entries":[{"filename":"Remote/Album/Song.flac","size":123}]}`
- **Request Body JSON (Flattened Format):** `{"username":"friend","filename":"Remote/One.mp3","size":7}`
- **Expected Response Status:** 200 OK
- **Key Assertions:**
  - Body contains "status" field (values: "partial" or "ready")
  - Body contains "count" field (numeric, total entry count)
  - Body contains "total_bytes" field (numeric sum of sizes)
  - Body contains "extension" field (extracted from filename e.g., "flac" from "Song.flac")
- **SessionCommand Sent:** None
- **Validation Requirements:**
  - "username" field is required (400 Bad Request if missing)
  - Supports both array format (entries array) and flattened format (single filename/size)
  - "complete" field defaults to true if not specified

---

# 7. OTHER ENDPOINTS (1 endpoint)

## 7.1 POST /api/v0/shares/rescan - Rescan Shares
- **HTTP Method & Path:** POST /api/v0/shares/rescan
- **Request Body JSON:** (empty)
- **Expected Response Status:** 202 Accepted
- **Key Assertions:**
  - Body contains "files" field (numeric count of files)
  - Response structure similar to share snapshot JSON
- **SessionCommand Sent:** None
- **Note:** This endpoint IS currently implemented in routing (line 2649)

---

# SUMMARY TABLE

| Category | Endpoint | Method | Status Code | Requires SessionCommand |
|----------|----------|--------|-------------|--------------------------|
| Searches | /api/v0/searches | POST | 201 | Yes (Search) |
| Searches | /api/v0/searches/{token}/complete | POST | 200 | No |
| Searches | /api/v0/searches/prune | POST | 200 | No |
| Searches | /api/v0/search-responses | POST | 200 | No |
| Transfers | /api/v0/transfers | POST | 201 | No |
| Transfers | /api/v0/transfers/{id}/start | POST | 200/409 | Yes (TransferPeer) |
| Transfers | /api/v0/transfers/{id}/progress | POST | 200 | No |
| Transfers | /api/v0/transfers/{id}/complete | POST | 200 | No |
| Messages | /api/v0/messages | POST | 201 | Yes (MessageUser) |
| Messages | /api/v0/messages/inbound | POST | 201 | No |
| Messages | /api/v0/messages/{id}/ack | POST | 200 | Yes (MessageAcked) |
| Rooms | /api/v0/rooms/refresh | POST | 202 | Yes (RefreshRooms) |
| Rooms | /api/v0/rooms/{name}/join | POST | 201 | Yes (JoinRoom) |
| Rooms | /api/v0/rooms/{name}/join | DELETE | 200 | Yes (LeaveRoom) |
| Rooms | /api/v0/rooms/{name}/messages | POST | 200 | Yes (SayRoom) |
| Users | /api/v0/users/watch | POST | 201 | Yes (WatchUser) |
| Users | /api/v0/users/{username}/watch | DELETE | 200 | Yes (UnwatchUser) |
| Users | /api/v0/users/{username}/stats/request | POST | 202 | Yes (RequestUserStats) |
| Users | /api/v0/users/{username}/browse/request | POST | 202 | Yes (BrowseUser) |
| Users | /api/v0/users/{username}/browse/folder | POST | 202 | Yes (BrowseFolder) |
| Users | /api/v0/users/{username}/browse/fail | POST | 200 | No |
| Browse | /api/v0/browse-responses | POST | 200 | No |

**Total Endpoints Requiring Implementation: 22**
**Endpoints with SessionCommand: 13**
**Endpoints without SessionCommand: 9**

