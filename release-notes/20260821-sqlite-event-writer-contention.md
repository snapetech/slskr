---
category: fixed
audience: users, operators
area: persistence
action: none
breaking: false
---
Concurrent Web UI and API activity no longer causes SQLite event-journal lock contention to appear as a false Soulseek session failure; any persistence error is logged separately while the connected session remains healthy.
