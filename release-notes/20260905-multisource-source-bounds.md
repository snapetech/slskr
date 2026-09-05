---
category: security
audience: users, operators
area: multisource-api
action: none
breaking: false
---
Versioned multisource swarm requests now reject source arrays above the executor limit before cloning and deserializing them, and mesh-discovered source lists are capped to that same limit.
