---
category: security
audience: users, operators
area: relay
action: none
breaking: false
---

Relay multipart handling now borrows uploaded binary sections instead of duplicating the request body in memory, while bounding the number of accepted sections.
