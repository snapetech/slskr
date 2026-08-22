---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

Guarded Rust commands now use their documented 12 GiB virtual-memory ceiling
without also inheriting the separate 4 GiB browser/Node process cgroup cap.
This prevents ordinary Cargo formatting, checking, and builds from being
terminated by the unrelated non-Rust process limit.
