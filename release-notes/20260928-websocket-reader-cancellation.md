---
category: fixed
audience: operators
area: runtime-lifecycle
action: none
breaking: false
---
Event, SignalR, and relay WebSocket readers now run within their connection future, so cancellation drops pending reads with the connection. Existing bounded frame queues still drain on reader completion. Track-processing requests use daemon task ownership and return 503 when task admission has closed.
