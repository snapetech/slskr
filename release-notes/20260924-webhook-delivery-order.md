---
category: changed
audience: users
area: webhooks
action: none
breaking: false
---
Webhook delivery logs and delivery statistics now share one persistence order
with webhook creation, activation, and deletion. A late delivery update cannot
restore stale webhook statistics after a configuration change.
