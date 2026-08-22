---
category: changed
audience: operators
area: parity-evidence
action: none
breaking: false
---

All direct Cargo commands now enter the same hard process-memory limit as other
repository proof commands, including rustc processes launched through Cargo's
compiler wrapper, in addition to the one-job Rust virtual-memory guard.
