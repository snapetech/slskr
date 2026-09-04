---
category: fixed
audience: operators
area: relay-shares
action: none
breaking: false
---

Relay share database uploads now use a restart-stable storage name, reject
share metadata that cannot round-trip through SQLite, preserve share-count
invariants, and refuse manifest writes larger than the configured read limit.
