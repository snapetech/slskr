---
category: fixed
audience: operators
area: distributed-state
action: Repair corrupt distributed depth rows if a load now reports an invalid value.
breaking: false
---
Distributed tree and child state loading now rejects negative or oversized persisted depths instead of converting them to misleading values.
