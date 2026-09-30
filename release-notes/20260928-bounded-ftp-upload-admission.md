---
category: fixed
audience: operators
area: integrations
action: none
breaking: false
---
Completed-download FTP uploads now admit at most 64 jobs, with four active uploads per daemon. Full admission applies backpressure before creating a worker. Daemon shutdown closes admission, wakes waiting callers, and cancels owned uploads. Existing FTP retry and transfer settings still apply.
