---
category: changed
audience: users, operators
area: realtime-hubs
action: none
breaking: false
---
Target-compatible web clients can now use the real SignalR JSON WebSocket transport for application, logs, search, SongID, listening-party, and transfer updates, including hub invocations and automatic reconnect. Hub negotiation advertises only the transport implemented by slskR. Native message and room notifications continue to use slskR's event feed.
