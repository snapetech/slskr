---
category: changed
audience: users
area: file-transfers
action: none
breaking: false
---
File-transfer payload writes no longer flush after every chunk; handshake token
and offset writes retain their required flush boundaries.
