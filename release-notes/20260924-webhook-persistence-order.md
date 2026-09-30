---
category: changed
audience: users
area: webhooks
action: none
breaking: false
---
Webhook creation, activation changes, and deletion now share a persistence
order. A delayed activation write cannot restore a webhook deleted while it was
pending.
