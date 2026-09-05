---
category: security
audience: users, operators
area: event-streams
action: none
breaking: false
---
Live event broadcasts and the TypeScript WebSocket SDK now enforce the same bounded message handling as the Go client and persisted event store, preventing oversized event payloads from entering live queues or being dispatched to SDK listeners.
