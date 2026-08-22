---
category: fixed
audience: users, operators
area: soulseek-session
action: none
breaking: false
---

Soulseek server frames that arrive in multiple TCP reads no longer desynchronize the session when a receive operation is interrupted; the daemon now retains partial frame bytes and avoids false reconnects.
