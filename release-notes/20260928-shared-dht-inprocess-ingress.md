---
category: fixed
audience: operators
area: shared-peer-gateway
action: none
breaking: false
---
Shared DHT packets now retain their original peer address through bounded in-process delivery. Mainline reuses the public UDP socket without a dedicated receive or forwarding port. Oversized packets and full or closed queues are rejected. The install guide also confirms peer TCP sharing works with DHT disabled.
