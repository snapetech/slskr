---
category: fixed
audience: operators
area: live-interop
action: none
breaking: false
---

The slskdN distributed-peer interoperability check now waits up to 60 bounded
seconds by default for the target's NetInfo and parent state, while running
file-transfer checks before that wait so short-lived test endpoint overrides
remain valid.
