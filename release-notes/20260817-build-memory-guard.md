---
category: changed
audience: operators
area: build-safety
action: none
breaking: false
---

The Rust build guard now disables dev debug metadata and incremental
compilation by default, reducing peak LLVM memory while retaining the
exclusive one-job and virtual-memory limits.
