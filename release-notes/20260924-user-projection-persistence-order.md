---
category: changed
audience: users
area: users
action: none
breaking: false
---
User watch changes and incoming user status/statistics projections now share one
persistence order. A delayed watch or unwatch write can no longer restore an
older watched state in SQLite.
