---
category: fixed
audience: users, operators
area: transfers
action: none
breaking: false
---

Transfer delete requests now surface filesystem removal failures instead of returning success after leaving the local file behind; already-removed files remain treated as successful cleanup.
