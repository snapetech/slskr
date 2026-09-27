---
category: changed
audience: users
area: interests
action: none
breaking: false
---
Liked and hated interest changes now share a persistence order. A pending write
cannot restore an interest after a newer deletion, and a failed database write
leaves the in-memory interest list unchanged.
