---
category: fixed
audience: users, operators
area: mesh-sync
action: none
breaking: false
---

Mesh synchronization now accepts its validated maximum inbound batch size atomically and reports merge failures as failed synchronization work instead of acknowledging a silent zero-entry success.
