---
category: changed
audience: operators
area: performance
action: none
breaking: false
---
The Rust Web now shares transfer-speed responses requested by the active route and player for up to 200 ms, avoiding duplicate API reads during page loading. No operator configuration change is required.
