---
category: fixed
audience: operators
area: mesh-overlay
action: none
breaking: false
---

Inbound mesh service calls now use the complete overlay validator before
dispatch, including service and method field bounds/control-character checks
and the negotiated frame payload limit.
