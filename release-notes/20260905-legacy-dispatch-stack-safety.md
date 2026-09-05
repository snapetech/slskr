---
category: fixed
audience: operators
area: release-ops
action: none
breaking: false
---
Legacy-route diagnostic mode now sends hash-sync and shadow-index merge requests through the stack-safe bounded dispatcher, preventing large valid batches from aborting the worker with a stack overflow.
