---
category: fixed
audience: operators
area: live-validation
action: none
breaking: false
---

The VPN-backed public interop matrix now waits through the guarded Rust build hand-off, reuses the verified binary, selects non-colliding account/profile pairs, retries unsolicited public-port handshakes, and keeps the negative indirect case offline. This prevents false failures while NAT-PMP ports are claimed and false green results from unusable listener metadata.
