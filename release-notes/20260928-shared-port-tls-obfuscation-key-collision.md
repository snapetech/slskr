---
category: fixed
audience: users
area: peer-listener
action: none
breaking: false
---
Random Soulseek obfuscated initialization keys avoid the TLS record prefix used by the mesh gateway on the shared port, preventing locally generated peer connections from being misclassified as mesh TLS traffic.
