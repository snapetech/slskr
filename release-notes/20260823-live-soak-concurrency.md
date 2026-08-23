---
category: fixed
audience: operators
area: live-validation
action: none
breaking: false
---

The public Soulseek soak now keeps its server-event loop and plain/obfuscated listener accepts responsive while unrelated peer connection attempts are slow, malformed, or filtered. Valid direct, obfuscated, and indirect handshakes are no longer delayed behind stale public traffic during VPN-backed acceptance runs.
