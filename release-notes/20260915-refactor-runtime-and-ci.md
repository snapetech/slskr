---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---

Managed indirect peer requests now time out instead of holding the connection manager indefinitely; event WebSocket clients can use bounded subscription filters; CI and release checks enforce locked dependencies; and Web/dashboard polling avoids redundant fan-out.
