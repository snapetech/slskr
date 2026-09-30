---
category: security
audience: users
area: peer-listener
action: none
breaking: false
---
Unknown initialization extensions on the shared peer and mesh port now use the same size bound as known peer initialization. Oversized advertised bodies are rejected from their headers.
