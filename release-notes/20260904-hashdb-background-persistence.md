---
category: fixed
audience: users, operators
area: hashdb
action: none
breaking: false
---

Hashes learned by backfill, completed-file metadata processing, and mesh
synchronization now update the durable HashDb projection as well as the local
state file, so those discoveries survive restart when SQLite persistence is
enabled.
