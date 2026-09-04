---
category: fixed
audience: users, operators
area: websocket-and-relay-transport
action: none
breaking: false
---
Upgraded websocket reads, SignalR keepalive traffic, and relay-agent connection setup now have bounded deadlines, so stalled peers or controllers cannot hold a connection task indefinitely.
