---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
On graceful shutdown or restart, the daemon now saves its latest distributed-tree snapshot after managed workers stop, so a queued revision is not left behind when the persistence worker is canceled. If the final write fails or times out, the daemon logs the error.
