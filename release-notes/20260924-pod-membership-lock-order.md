---
category: changed
audience: operators
area: room-and-pod-membership
action: none
breaking: false
---
Membership acceptance now waits for room state before locking pending-request
state, keeping pending-request reads available while room updates wait. A
successful acceptance updates room membership and the member role together.
