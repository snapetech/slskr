---
category: changed
audience: operators
area: acceptance-evidence
action: none
breaking: false
---

Guarded distributed-network probes now record the frozen target's response
separately from the replacement's request. This prevents a successful
bidirectional distributed exchange from being reported as one-way evidence.
