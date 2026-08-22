---
category: changed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

Guarded production Rust checks now use 1024 codegen units by default, keeping
the monolithic daemon compiler's working set within the repository memory
ceiling without changing the runtime panic profile.
