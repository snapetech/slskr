---
category: fixed
audience: users
area: wishlist-search
action: none
breaking: false
---
Concurrent Lidarr wanted syncs now check the current wishlist while holding its persistence turn. Each page's item changes are saved atomically, and an SQLite failure rolls back that page's in-memory changes.
