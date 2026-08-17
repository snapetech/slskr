---
category: changed
audience: operators
area: security-controls
action: none
breaking: false
---

Overlay admission now enforces bounded per-IP, global, message, and request
budgets. Persisted security bans are checked before overlay authentication, and
Soulseek search and browse routes return `429` after their configured safety
windows are exhausted.
