---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

Cargo commands launched from a process-guarded runner now move into a separate
12 GiB Rust systemd unit instead of inheriting the 4 GiB application/browser
cgroup. This prevents rustfmt, rustc, and other Rust tooling from being killed
by an unrelated process-memory ceiling while retaining bounded execution.
