---
category: changed
audience: users, operators
area: virtualsoulfind-v2
action: Pin the compatibility profile and update clients using the slskR-only v2 endpoint.
breaking: true
---

Compatibility profiles now match the frozen targets: the slskd profile returns
404 for VirtualSoulfind v2 routes, and the slskdN profile returns the pinned
target's 503 disabled response even when the target-style YAML option is set.
