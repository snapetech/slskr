---
category: changed
audience: operators
area: shared-udp-transport
action: none
breaking: false
---

When the frozen sharing predicate is active, slskR now owns the public DHT UDP
port in its bounded gateway demux, forwards DHT-shaped datagrams to an internal
mainline endpoint, and returns DHT-shaped responses through the public source
port while retaining overlay and QUIC traffic routing.
