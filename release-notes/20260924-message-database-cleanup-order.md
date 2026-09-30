---
category: changed
audience: users
area: messaging
action: none
breaking: false
---
Manual database cleanup now removes expired messages from both SQLite and live
conversation history under one persistence order. If SQLite cleanup fails,
the live history is restored and cleanup reports an error.
