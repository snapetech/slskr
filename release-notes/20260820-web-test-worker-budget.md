---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

Web test commands now run Vitest with one worker by default. This keeps the
aggregate Node test process within the repository's bounded process-memory
guard instead of allowing parallel workers to exhaust the cap.
