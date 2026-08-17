---
category: changed
audience: operators
area: shared-udp-transport
action: none
breaking: false
---

Shared DHT mode now routes mainline outbound requests and responses through
the public UDP socket, preserving the configured source port while retaining a
bounded private receive endpoint for gateway demultiplexing.
