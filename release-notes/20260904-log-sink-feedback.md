---
category: fixed
audience: operators
area: logging
action: none
breaking: false
---

Disk and Loki log-sink failures are now reported on stderr with their cause
instead of being silently discarded, making degraded diagnostics visible.
