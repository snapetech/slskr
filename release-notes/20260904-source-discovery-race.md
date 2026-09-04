---
category: fixed
audience: users, operators
area: source-discovery
action: none
breaking: false
---

Source discovery start and stop requests now reserve and track their lifecycle
atomically, preventing overlapping requests or delayed searches from changing
the state of a newer discovery run.
