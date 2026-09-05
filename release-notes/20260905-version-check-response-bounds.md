---
category: fixed
audience: users, operators
area: version-check
action: none
breaking: false
---
Startup and forced version checks now cap the GitHub release response before parsing it, preventing an unexpectedly large collector response from consuming unbounded memory.
