---
category: fixed
audience: operators
area: build-and-test-memory
action: Run browser and Node parity/build commands through the repository process-memory guard.
breaking: false
---

Parity and release-gate browser/Node subprocesses now run under a hard 4 GiB memory ceiling with a portable virtual-memory fallback, preventing an unbounded Playwright or frontend subprocess from exhausting the host.
