---
category: fixed
audience: operators
area: runtime-lifecycle
action: none
breaking: false
---
Daemon shutdown now closes forwarding listeners and their connection workers, releases permits, and rejects late forwarding rules. Forwarding pumps stay within their connection's lifetime. Share-rescan, administrative webhook, script-dispatch, and completed-download FTP jobs now share the daemon's task registry and shutdown cancellation.
