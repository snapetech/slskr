---
category: fixed
audience: operators
area: validation
action: none
breaking: false
---

Listener parity checks now report released ports only while their closure is authoritative, avoiding false mismatches when an unrelated process later reuses a port.
