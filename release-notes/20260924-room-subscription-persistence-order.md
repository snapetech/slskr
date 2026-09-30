---
category: changed
audience: users
area: rooms
action: none
breaking: false
---
Room joins and leaves now share one persistence order across API routes and
compatibility handlers. A queued room change cannot restore a subscription
after a later leave.
