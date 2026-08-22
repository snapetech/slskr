---
category: changed
audience: operators
area: acceptance-evidence
action: none
breaking: false
---

The guarded interop runner now records type-1 obfuscated peer probes separately
from regular traffic. The frozen target accepts the obfuscated request but
returns a plain response, so the harness verifies that compatibility behavior;
its loopback endpoint override is test-only.
