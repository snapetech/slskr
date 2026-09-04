---
category: fixed
audience: operators
area: mesh-dht
action: none
breaking: false
---

Mesh DHT storage no longer leaks admission quota when a store is rejected or
when a key is replaced, and publication records reject MessagePack lengths
that cannot be represented instead of emitting malformed records.
