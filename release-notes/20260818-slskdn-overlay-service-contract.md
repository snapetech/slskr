---
category: fixed
audience: users, operators
area: overlay-service
action: none
breaking: false
---

The slskdN compatibility profile now returns the pinned target's
`Service '<name>' not found` response for remote `pods`, `private-gateway`, and
`shadow-index` calls, matching the frozen runtime's registered service set.
Local HTTP pod and VirtualSoulfind controllers are unchanged.
