---
category: security
audience: operators
area: multisource-api
action: none
breaking: false
---
Versioned multisource swarm requests now enforce the same file, chunk, and discovered-source limits as direct multisource execution, preventing oversized compatibility requests from creating invalid jobs or excess runtime state.
