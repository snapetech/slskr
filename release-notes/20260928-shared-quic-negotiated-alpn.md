---
category: fixed
audience: operators
area: shared-peer-gateway
action: none
breaking: false
---
Native QUIC control and data now use one endpoint on the shared peer socket. TLS negotiates the protocol after handshake reassembly, supporting fragmented ClientHellos and both protocols from one peer socket. Disabled protocols are rejected by TLS. Pending handshakes, buffers, ingress queues, and inbound streams remain bounded.
