---
category: fixed
audience: users, operators
area: transfer-persistence
action: none
breaking: false
---

Transfer history is now preserved across daemon restarts. Event files are
validated before use and transfer updates are flushed and synchronized before
the daemon reports them as durable.
