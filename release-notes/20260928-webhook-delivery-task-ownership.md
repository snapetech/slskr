---
category: fixed
audience: operators
area: integrations
action: none
breaking: false
---
Event webhook and compatibility webhook delivery workers now share daemon task ownership and cancellation. Delivery concurrency limits remain in place. Closed task admission rejects new deliveries, releases their permits, and records ordinary webhook deliveries as failed instead of leaving them queued.
