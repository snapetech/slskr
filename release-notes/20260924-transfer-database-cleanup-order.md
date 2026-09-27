---
category: changed
audience: users
area: transfers
action: none
breaking: false
---
Manual database cleanup now removes terminal transfer rows from SQLite with
tombstones as well as from the live transfer list. If SQLite rejects the
delete, the live entries are restored and cleanup reports an error.
