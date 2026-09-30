---
category: changed
audience: users
area: events
action: none
breaking: false
---
API-injected events and background event records now persist in one order.
Event-history reads stay available during SQLite writes, and a failed API event
write rolls back its matching in-memory snapshot.
