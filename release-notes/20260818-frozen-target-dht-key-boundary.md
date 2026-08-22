---
category: changed
audience: operators
area: acceptance-evidence
action: none
breaking: false
---

The universal parity record now identifies the pinned slskdN reverse-routing
blocker: its peer resolver uses 32-byte hashed keys while its remote mesh DHT
store service accepts only 20-byte keys, so target-originated peer routing
cannot be certified without changing the frozen target.
