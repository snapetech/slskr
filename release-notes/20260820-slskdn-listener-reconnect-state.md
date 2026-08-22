---
category: fixed
audience: operators
area: controller-compatibility
action: none
breaking: false
---
The slskdN profile now reports a pending reconnect after a connected watched
listener-address or listener-port change, while preserving the frozen
profile's live socket and advertisement boundaries until restart.
