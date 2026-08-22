---
category: fixed
audience: operators
area: controller-compatibility
action: none
breaking: false
---
The slskdN disaster-mode status endpoint now reports its runtime coordinator
state instead of inferring a fallback level from `force`, `auto`, or a
disconnected startup session. A fresh process therefore reports the frozen
target's Normal state until a runtime transition occurs.
