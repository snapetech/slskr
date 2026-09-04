---
category: fixed
audience: users, operators
area: share-configuration
action: none
breaking: false
---

Adding a share through the compatibility configuration endpoint now validates
the path, updates the live share settings, rescans the index, and reports
duplicate or scan failures instead of returning a false success.
