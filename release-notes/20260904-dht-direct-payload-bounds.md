---
category: security
audience: users, operators
area: mesh-dht
action: none
breaking: false
---
The in-process mesh DHT handler now enforces the overlay payload limit even when called directly, keeping its public service boundary consistent with the network gateway.
