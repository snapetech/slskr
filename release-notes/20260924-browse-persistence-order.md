---
category: changed
audience: users
area: browse
action: none
breaking: false
---
Concurrent browse requests, responses, failures, and cancellations now persist in
the same order as their in-memory changes, preventing SQLite from restoring an
older browse status after a newer update.
