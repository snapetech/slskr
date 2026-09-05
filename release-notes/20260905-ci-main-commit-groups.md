---
category: changed
audience: operators
area: ci
action: none
breaking: false
---
Main-branch pushes now use commit-specific CI concurrency groups, so a queued result cannot be displaced by the next fix push and each commit can complete the checks required by release gating.
