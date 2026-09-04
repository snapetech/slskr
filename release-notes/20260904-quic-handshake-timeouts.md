---
category: security
audience: users, operators
area: quic-transport
action: none
breaking: false
---
QUIC control and data listeners now bound incoming handshakes to the transport connection timeout, preventing an incomplete peer handshake from monopolizing the accept loop.
