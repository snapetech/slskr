---
category: fixed
audience: operators
area: runtime-lifecycle
action: none
breaking: false
---
Relay connections reserve bounded cleanup capacity before registration. Cancellation removes the live sender immediately and queues protocol deregistration through a daemon-owned worker. Shutdown rejects new reservations and clears live connection/request state while retaining completed share-upload history.
