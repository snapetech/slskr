---
category: changed
audience: operators
area: build-safety
action: none
breaking: false
---

Repository Rust commands now use an exclusive build lock, one Cargo/test job, and a bounded virtual-memory limit so concurrent or over-parallelized build entrypoints fail instead of exhausting host memory.
