---
category: changed
audience: operators
area: shared-peer-gateway
action: Remove separate native/current overlay or obfuscation binds, or match the peer bind; forward its port for TCP and UDP.
breaking: true
---
Native/current DHT, overlay, and QUIC now share the peer listener's port without QUIC backend or forwarding sockets. Bounded queues preserve packet metadata and expire idle sessions. Real QUIC Initial selection fixes v1/v2 key derivation and distinguishes DHT from binary short headers. LAN-only DHT serves local peers. One-shot QUIC sends finish cleanup before returning.
