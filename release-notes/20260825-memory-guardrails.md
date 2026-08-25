---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---

Rust formatting now checks each source file under a hard 4 GiB no-swap process limit, while compiler commands retain the separate 12 GiB ceiling; a pathological formatter allocation can fail safely without consuming host memory.
