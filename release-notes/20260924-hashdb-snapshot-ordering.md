---
category: changed
audience: users
area: hashdb
action: none
breaking: false
---
HashDb updates from API merges, mesh sync, backfill, and transfer hashing now
share one persistence order. A delayed snapshot can no longer erase a newer
entry or leave memory ahead of the data restored after restart.
