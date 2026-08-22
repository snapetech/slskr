---
category: changed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

The production HTTP dispatcher is now lowered as bounded route groups, keeping
fresh slskR builds below the repository's hard process-memory ceiling. The
retained monolithic dispatcher is diagnostic-only and is rejected by the build
guard when selected explicitly or through `--all-features`.
