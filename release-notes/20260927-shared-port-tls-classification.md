---
category: fixed
audience: users
area: peer-listener
action: none
breaking: false
---
The shared peer and mesh port now checks a complete TLS ClientHello record prefix before routing to the mesh gateway, preserving valid Soulseek initialization frames that begin with the same two bytes.
