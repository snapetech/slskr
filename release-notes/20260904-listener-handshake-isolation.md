---
category: fixed
audience: users, operators
area: soulseek-listener-lifecycle
action: none
breaking: false
---
Soulseek listener handshakes now run in bounded per-connection tasks, so a stalled peer cannot block new inbound connections or the listener control loop.
