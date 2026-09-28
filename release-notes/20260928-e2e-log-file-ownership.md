---
category: fixed
audience: operators
area: e2e-harness
action: none
breaking: false
---
Native E2E nodes now drain log pipes and close their file handles during joined cleanup, including failed process launch. In-memory diagnostics are bounded and socket diagnostics have deadlines. Contacts tests observe the real response before navigation and fail when it is missing or malformed.
