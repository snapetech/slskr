---
category: fixed
audience: users
area: shared-media-streaming
action: none
breaking: false
---
Owner share grants now enforce and persist an explicit maxConcurrentStreams value from 1 through 64. Excess ticketed streams receive 429 until capacity is released. Updating the limit preserves share permissions; null clears it, and grants without an explicit limit retain their existing behavior.
