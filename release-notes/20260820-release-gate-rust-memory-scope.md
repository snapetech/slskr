---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

The release gate no longer places Cargo formatting, checks, clippy, or tests
inside the separate 4 GiB browser/Node cgroup. Frontend and Node steps remain
process-guarded, while Rust steps use the one-job 12 GiB virtual-memory guard.
