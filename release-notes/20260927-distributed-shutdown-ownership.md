---
category: fixed
audience: operators
area: daemon-lifecycle
action: none
breaking: false
---
Distributed parent and child socket loops now belong to the daemon's managed shutdown registry, so shutdown closes and joins them before persisting the final distributed network snapshot.
