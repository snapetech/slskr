---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
Windows release archives now let the nested Rust guard apply its larger serialized build ceiling instead of rejecting Cargo inside the outer process guard's smaller fallback limit.
