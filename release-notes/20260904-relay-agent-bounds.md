---
category: fixed
audience: users, operators
area: relay
action: none
breaking: false
---

Relay agents now bound inbound SignalR frame size, message count, upload filename/token fields, and concurrent uploads so malformed or bursty controller traffic cannot grow work without limit.
