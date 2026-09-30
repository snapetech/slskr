---
category: fixed
audience: operators
area: shared-peer-gateway
action: none
breaking: false
---
Tunnel readers and QUIC proxy workers now remain joined to daemon shutdown and are cancelled when their local owners close. Late work is rejected during shutdown. Overlay metadata cleanup runs when guards drop, including outside Tokio, and shutdown clears gateway connection records after joining workers. The public transport continues to share its existing port.
