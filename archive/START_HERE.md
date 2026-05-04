# POST Endpoints Implementation - START HERE

## Quick Overview

Five comprehensive analysis documents have been created to help you implement POST endpoints correctly. All code patterns have been analyzed from the existing codebase.

## Document Guide (Read in This Order)

### 1. **ANALYSIS_README.txt** (2-3 minutes) ← START HERE
Quick summary of what's been analyzed:
- Overview of all 4 main documents
- Key findings summary
- Next steps checklist

### 2. **IMPLEMENTATION_GUIDE.md** (5-10 minutes) ← READ SECOND
Quick start guide with:
- Key insights summary
- Step-by-step checklist for each endpoint
- List of 6 endpoints to implement
- Common mistakes to avoid
- Complete working example

### 3. **CODE_PATTERNS_ANALYSIS.md** (15-20 minutes) ← DEEP DIVE
Comprehensive reference with:
- All store creation methods explained (SearchStore, TransferQueue, MessageStore, etc.)
- JSON field extraction functions documented
- SessionCommand enum variants
- Route handler pattern explained
- 11 detailed sections with examples

### 4. **FUNCTION_SIGNATURES_REFERENCE.md** (During coding)
Copy-paste ready reference with:
- All function signatures with line numbers
- Parameter descriptions
- Usage examples
- Ready-to-use code snippets
- Template for implementing new handlers

### 5. **EXACT_CODE_TO_ADD.md** (During coding) ← USE THIS FOR COPY-PASTE
Actual code ready to copy-paste:
- `extract_json_u64_field()` function
- `conflict_response()` function
- Path extraction helper functions
- All 6 POST endpoint implementations
- Where to add each piece

### Bonus Documents:

- **POST_ENDPOINTS_QUICK_REF.md** - Quick patterns for each endpoint type
- **ENDPOINT_IMPLEMENTATION_CHECKLIST.md** - Detailed checklist (if created)

---

## The 3-Step Process

### Step 1: Understand the Pattern (5 minutes)
Read IMPLEMENTATION_GUIDE.md sections:
- "1. All Stores Follow This Pattern"
- "2. There Are Two JSON Extraction Functions You Need"
- "3. SessionCommand Enum Already Has Everything You Need"

### Step 2: Implement the Foundation (2 minutes)
Add from EXACT_CODE_TO_ADD.md Section 1:
- `extract_json_u64_field()` function (essential)
- Path extraction helper functions (needed for dynamic routes)
- `conflict_response()` function (optional but recommended)

### Step 3: Add Endpoints (30-60 minutes)
Add from EXACT_CODE_TO_ADD.md Section 4, one at a time:
1. POST /api/v0/transfers
2. POST /api/v0/messages
3. POST /api/v0/users/{username}/watch
4. POST /api/v0/browse/{username}
5. POST /api/v0/rooms/{name}/join
6. POST /api/v0/transfers/{id}/start

Test each with `cargo test` after adding.

---

## Key Findings At a Glance

### JSON Field Extraction
```rust
// These already exist:
extract_json_string_field(body, "field")     // From utils.rs
extract_json_u32_field(body, "field")        // From main.rs
extract_json_bool_field(body, "field")       // From main.rs

// You need to add:
extract_json_u64_field(body, "field")        // Copy from u32 version
```

### All Stores Work the Same Way
```rust
let mut store = state.store_name.write().await;
let record = store.create(...);  // or .add(...)
drop(store);
Ok(created_response(record.json()))
```

### Session Commands Already Exist
```rust
send_session_command(state, SessionCommand::WatchUser(username)).await.ok();
```

All variants are pre-defined. Just use them.

### Response Types Ready to Use
```rust
created_response(body)              // 201 Created
bad_request_response(message)       // 400 Bad Request
not_found_response()                // 404 Not Found
conflict_response(message)          // 409 Conflict (add this)
```

---

## Code Locations Quick Reference

| Item | Line(s) | File |
|------|---------|------|
| Route handler function | 2471-2750 | main.rs |
| Match statement | 2489+ | main.rs |
| extract_json_u32_field() | 2814-2820 | main.rs |
| extract_json_bool_field() | 2822-2833 | main.rs |
| → Where to add u64 version | After 2833 | main.rs |
| send_session_command() | 4755-4761 | main.rs |
| SessionCommand enum | 2377-2422 | main.rs |
| SearchStore::create() | 478-504 | main.rs |
| TransferQueue::create() | 993-1020 | main.rs |
| MessageStore::add() | 2032-2047 | main.rs |
| UserStore::watch() | 1472-1497 | main.rs |
| RoomStore::join() | 2203-2223 | main.rs |
| BrowseStore::request() | 1705-1733 | main.rs |

---

## What Gets Implemented

### 6 Endpoints Total

1. **POST /api/v0/transfers** - Create transfer entry
   - Stores: filename, direction, peer_username, local_path, size
   - Returns: 201 Created with TransferEntry

2. **POST /api/v0/messages** - Create message
   - Stores: username, direction ("in"/"out"), body
   - Returns: 201 Created with MessageRecord

3. **POST /api/v0/users/{username}/watch** - Watch user
   - Creates/updates user record with watched=true
   - Sends SessionCommand::WatchUser
   - Returns: 201 Created with UserRecord

4. **POST /api/v0/browse/{username}** - Browse user shares
   - Creates/updates browse record with status="requested"
   - Sends SessionCommand::BrowseUser
   - Returns: 201 Created with BrowseRecord

5. **POST /api/v0/rooms/{name}/join** - Join room
   - Creates/updates room record with joined=true
   - Sends SessionCommand::JoinRoom
   - Returns: 201 Created with RoomRecord

6. **POST /api/v0/transfers/{id}/start** - Start transfer
   - Updates transfer status from "queued" to "in_progress"
   - Returns: 200 OK with updated TransferEntry
   - Returns: 404 if not found, 409 if not queued

---

## File Changes Summary

- **main.rs**: Add ~120 lines
  - 1 function: extract_json_u64_field()
  - 4 helper functions: Path extraction
  - 6 route handlers: POST endpoints

- **routing.rs**: Add ~8 lines (optional)
  - 1 function: conflict_response()

---

## Testing

Each endpoint has tests in the test suite that verify:
- POST requests work correctly
- JSON fields are extracted properly
- Records are created with correct values
- Responses have correct status codes
- Event logging works

Run tests: `cargo test`

---

## Common Mistakes (Read Before Coding)

1. **Forgetting to drop() the lock** - Causes deadlocks
2. **Not awaiting send_session_command()** - Will not compile
3. **Not handling Option from extract functions** - Type mismatch
4. **Not escaping strings in JSON** - Security issue
5. **Wrong response function** - Returns wrong status

See IMPLEMENTATION_GUIDE.md "Common Mistakes to Avoid" for details.

---

## Next Action

1. Open ANALYSIS_README.txt first (quick overview)
2. Then open IMPLEMENTATION_GUIDE.md (understand patterns)
3. Then open EXACT_CODE_TO_ADD.md (copy code)
4. Start coding with FUNCTION_SIGNATURES_REFERENCE.md nearby

All documents are in the project root directory.

**Time estimate to complete all 6 endpoints: 1-2 hours**

Good luck! All the patterns are already in the codebase - you're just extending them.
