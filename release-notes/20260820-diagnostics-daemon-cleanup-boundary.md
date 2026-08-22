---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---
The diagnostics differential harness now bounds daemon shutdown and force-cleans a test process that ignores SIGTERM, so guarded release checks cannot hang during cleanup after a completed dump probe.
