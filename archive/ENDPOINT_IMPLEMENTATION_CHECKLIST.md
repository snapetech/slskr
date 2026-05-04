# Endpoint Implementation Checklist

## Quick Reference for Implementation Priority

### CRITICAL - Core Functionality (13 endpoints with SessionCommand)
These endpoints must send SessionCommand to the session manager:

- [ ] POST /api/v0/searches → SessionCommand::Search
- [ ] POST /api/v0/messages → SessionCommand::MessageUser
- [ ] POST /api/v0/messages/{id}/ack → SessionCommand::MessageAcked
- [ ] POST /api/v0/rooms/refresh → SessionCommand::RefreshRooms
- [ ] POST /api/v0/rooms/{name}/join → SessionCommand::JoinRoom
- [ ] DELETE /api/v0/rooms/{name}/join → SessionCommand::LeaveRoom
- [ ] POST /api/v0/rooms/{name}/messages → SessionCommand::SayRoom
- [ ] POST /api/v0/users/watch → SessionCommand::WatchUser
- [ ] DELETE /api/v0/users/{username}/watch → SessionCommand::UnwatchUser
- [ ] POST /api/v0/users/{username}/stats/request → SessionCommand::RequestUserStats
- [ ] POST /api/v0/users/{username}/browse/request → SessionCommand::BrowseUser
- [ ] POST /api/v0/users/{username}/browse/folder → SessionCommand::BrowseFolder
- [ ] POST /api/v0/transfers/{id}/start → SessionCommand::TransferPeer (conditional)

### STANDARD - Data Management (9 endpoints without SessionCommand)
These endpoints manage local state only:

- [ ] POST /api/v0/searches/{token}/complete
- [ ] POST /api/v0/searches/prune
- [ ] POST /api/v0/search-responses
- [ ] POST /api/v0/transfers
- [ ] POST /api/v0/transfers/{id}/progress
- [ ] POST /api/v0/transfers/{id}/complete
- [ ] POST /api/v0/messages/inbound
- [ ] POST /api/v0/users/{username}/browse/fail
- [ ] POST /api/v0/browse-responses

## Implementation Notes

### Path Parameter Extraction
Implement pattern matching for:
- `/api/v0/searches/{token}/complete` → Extract token from path
- `/api/v0/messages/{id}/ack` → Extract id from path
- `/api/v0/transfers/{id}/start` → Extract id from path
- `/api/v0/transfers/{id}/progress` → Extract id from path
- `/api/v0/transfers/{id}/complete` → Extract id from path
- `/api/v0/rooms/{name}/join` → Extract room name from path
- `/api/v0/rooms/{name}/messages` → Extract room name from path
- `/api/v0/users/{username}/watch` → Extract username from path
- `/api/v0/users/{username}/stats/request` → Extract username from path
- `/api/v0/users/{username}/browse/request` → Extract username from path
- `/api/v0/users/{username}/browse/folder` → Extract username from path
- `/api/v0/users/{username}/browse/fail` → Extract username from path

### JSON Field Extraction
Use existing helper functions:
- `extract_json_string_field(body, "field_name")`
- `extract_json_u32_field(body, "field_name")`
- `extract_json_bool_field(body, "field_name")`

### Error Responses
Implement proper error handling:
- 400 Bad Request for missing required fields
- 404 Not Found for non-existent resources
- 409 Conflict for policy violations (e.g., max transfer limit)

### Response Status Codes
- 200 OK - Standard successful update
- 201 Created - Resource creation
- 202 Accepted - Async command accepted
- 400 Bad Request - Validation error
- 404 Not Found - Resource not found
- 409 Conflict - Business rule violation

## Testing Guidance

All endpoints have corresponding tests in `crates/slskr/src/main.rs`:
- Search tests: Lines 6970-7285
- Transfer tests: Lines 7325-7540
- Message tests: Lines 8643-8708
- Room tests: Lines 8710-8785
- User tests: Lines 8212-8415
- Browse tests: Lines 8287-8448

Run tests with: `cargo test --test main`

