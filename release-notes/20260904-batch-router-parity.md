---
category: fixed
audience: users, operators
area: http-api
action: none
breaking: false
---
Batch API operations now use the same authenticated router as standalone requests, accept native JSON bodies from the Go and Python clients, honor per-operation timeouts, report actual HTTP statuses, and include total execution time.
