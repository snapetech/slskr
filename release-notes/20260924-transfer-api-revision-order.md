---
category: changed
audience: users
area: transfers
action: none
breaking: false
---
Transfer request-name, cancellation, retry, progress, and completion updates now
advance transfer revisions monotonically. This keeps SQLite from discarding an
update when multiple changes occur within the same millisecond or the clock
moves backward.
