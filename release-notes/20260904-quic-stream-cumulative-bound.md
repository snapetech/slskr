---
category: fixed
audience: operators
area: overlay-data
action: none
breaking: false
---

QUIC data streams now enforce their configured payload limit across the full
stream, including payloads written or read in multiple chunks.
