---
category: fixed
audience: users, operators
area: multisource-api
action: none
breaking: false
---
Versioned multisource swarm requests without an expected SHA-256 now fail before queue insertion instead of returning an asynchronous job that could never execute or complete.
