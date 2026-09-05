---
category: security
audience: users, operators
area: event-streams
action: none
breaking: false
---
Event WebSocket clients now share the 64 KiB frame boundary used by the Go and Python SDKs, while the server compacts oversized detail fields before sending framing overhead can exceed that limit.
