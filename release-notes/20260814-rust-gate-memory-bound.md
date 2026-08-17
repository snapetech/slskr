---
category: fixed
audience: operators
area: build-safety
action: none
breaking: false
---

Rust release-gate and live-interop fallback commands now run with bounded
virtual memory, limited build parallelism, and a bounded Rust thread stack so
formatting or compilation failures cannot consume the host's entire RAM and
swap budget.
