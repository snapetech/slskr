---
category: changed
audience: operators
area: security
action: none
breaking: false
---
Persisted username and IP ban changes now share one persistence order across
the security and overlay blocklist routes. Failed overlay blocklist writes
restore the in-memory ban state.
