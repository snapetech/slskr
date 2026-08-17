---
category: fixed
audience: operators
area: build-and-test-memory
action: Use the default focused controller test profile for guarded workspace tests.
breaking: true
---

The Rust build guard now rejects the historical monolithic controller-test profile before Cargo starts, preventing its known LLVM memory failure while keeping the bounded default workspace tests available.
