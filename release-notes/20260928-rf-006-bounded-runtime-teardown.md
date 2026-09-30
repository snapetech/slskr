---
category: changed
audience: operators
area: daemon
action: none
breaking: false
---
After managed async shutdown, the daemon gives Tokio blocking work five seconds to finish. A slow filesystem call cannot hold normal runtime teardown indefinitely; if the deadline expires, the daemon logs that remaining work may continue until its system call returns.
