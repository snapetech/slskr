---
category: changed
audience: users, operators
area: web-ui
action: none
breaking: false
---
Consolidated periodic Web UI refreshes behind one lifecycle-safe poller so mounted screens do not overlap requests or continue background polling while the document is hidden.
