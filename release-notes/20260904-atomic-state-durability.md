---
category: fixed
audience: operators
area: state-durability
action: none
breaking: false
---

Atomic share, transfer, pod, content, certificate-pin, and realm-index state
replacements now synchronize their parent directory after committing, reducing
the chance of losing a successful state update across an abrupt host restart.
