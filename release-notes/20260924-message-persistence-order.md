---
category: changed
audience: users
area: messaging
action: none
breaking: false
---
Message creation, acknowledgements, and conversation deletion now share one
persistence order across API routes and inbound session handling. A queued
message write cannot restore a conversation after it is deleted.
