---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Distributed network branch metadata and child depths now commit in one SQLite transaction, and persistence snapshots runtime state before database I/O. Existing installations need no configuration changes.
