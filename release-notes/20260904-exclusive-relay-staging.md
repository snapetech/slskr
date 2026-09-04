---
category: fixed
audience: operators
area: security
action: none
breaking: false
---

Relay-controller upload staging files are now created exclusively with private
permissions, preventing a pre-existing path or symlink from being truncated by
an incoming authenticated upload. The opened descriptor is retained through the
stream handoff, so later path replacement cannot redirect the streamed bytes.
