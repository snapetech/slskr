---
category: changed
audience: users, operators
area: protocol-transport
action: none
breaking: false
---

Server-message batches now stream frames one at a time, avoiding an aggregate buffer proportional to the entire batch.
