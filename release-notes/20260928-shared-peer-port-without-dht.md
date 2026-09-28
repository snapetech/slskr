---
category: fixed
audience: operators
area: shared-peer-gateway
action: none
breaking: false
---
Current native mesh TCP now shares the peer listener even when DHT is disabled. Gateway UDP keeps the same port number. CI runs a bounded live shutdown proof that checks listener ownership, TLS connection limits, client closure, and socket reuse, and retains its result with reproducibility metadata.
