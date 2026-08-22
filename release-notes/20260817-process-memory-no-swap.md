---
category: security
audience: operators
area: process-memory
action: none
breaking: false
---
Guarded repository commands now set a zero swap allowance alongside the hard RAM ceiling, preventing bounded builds and audits from shifting memory pressure into system swap while they run.
