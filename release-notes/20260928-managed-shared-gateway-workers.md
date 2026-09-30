---
category: fixed
audience: operators
area: shared-peer-gateway
action: none
breaking: false
---
Accepted TLS gateway handlers and UDP, QUIC, and DHT response workers now participate in joined daemon shutdown. Cancelling a handler or rejecting it during shutdown releases its connection admission and socket resources. The shared public listener and connection limits are preserved.
