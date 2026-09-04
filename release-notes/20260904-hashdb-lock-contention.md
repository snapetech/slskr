---
category: changed
audience: operators
area: hashdb
action: none
breaking: false
---

HashDb API writes no longer hold the in-memory discovery write lock while
waiting for SQLite persistence, keeping concurrent hash reads responsive when
the database is slow.
