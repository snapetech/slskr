---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

Frozen-target UI comparisons now isolate each workflow in a fresh browser
context and reject replacement surfaces that point at a frozen target UI root.
This keeps API-path evidence deterministic and prevents stale target assets
from being reported as replacement parity proof.
