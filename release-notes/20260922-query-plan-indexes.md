---
category: changed
audience: operators
area: database-query-plans
action: none
breaking: false
---
Common paged search-result, transfer, and webhook-log reads now have composite
indexes matching their filters and sort order, avoiding avoidable temporary
SQLite sort work as those tables grow.
