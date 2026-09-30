---
category: fixed
audience: operators
area: daemon-lifecycle
action: none
breaking: false
---
Delayed shutdown and restart commands now belong to the daemon's joined task registry. Cancellation releases pending command senders, and closed task admission prevents late scheduling. The response-flush delay and bounded Soulseek disconnect behavior are preserved.
