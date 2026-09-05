---
category: changed
audience: operators
area: ci
action: none
breaking: false
---
Main-branch CI runs are no longer canceled by a later push; pull-request runs still cancel superseded runs, so every pushed commit reaches a completed result for release gating.
