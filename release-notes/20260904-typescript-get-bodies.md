---
category: fixed
audience: users
area: client-sdk
action: none
breaking: false
---
Authenticated TypeScript client reads no longer send an invalid JSON body with
GET requests, restoring compatibility with Fetch implementations that reject
GET bodies.
