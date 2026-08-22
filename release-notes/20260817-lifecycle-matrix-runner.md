---
category: changed
audience: operators
area: parity-certification
action: none
breaking: false
---

Universal replacement certification now has a guarded lifecycle matrix runner
for both frozen profiles. It executes all 22 target/scenario cases serially,
rejects a replacement binary older than the current source, and requires
independent per-case evidence before any case can pass.
