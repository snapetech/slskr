---
category: fixed
audience: operators
area: webhooks
action: none
breaking: false
---
Frozen-compatible webhook delivery now bounds configured attempts to the
supported retry ceiling instead of allowing an unbounded delivery loop.
