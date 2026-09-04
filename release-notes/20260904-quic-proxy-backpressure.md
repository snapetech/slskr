---
category: fixed
audience: operators
area: overlay
action: none
breaking: false
---
The shared overlay UDP dispatcher now drops only excess packets for a congested QUIC proxy session instead of waiting on that session and delaying unrelated DHT and control traffic.
