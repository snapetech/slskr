---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
Controller parity and authentication release checks now choose isolated overlay ports and disable unused TLS listeners, so local validation is not blocked by an already-running slskR service.
