---
category: changed
audience: users
area: hashdb
action: none
breaking: false
---
Concurrent history-backfill requests now read and advance the progress cursor
under one persistence order. A later batch cannot overwrite an earlier
commit's cursor or leave older searches unprocessed.
