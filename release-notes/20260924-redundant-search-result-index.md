---
category: changed
audience: operators
area: database-query-plans
action: none
breaking: false
---
On startup, the daemon removes a redundant search-result index. SQLite's
existing `search_id` index already carries the integer row id ordering used by
paged reads, so the duplicate index added storage and update work without
improving the query plan. Transfer and webhook-log composite indexes remain.
