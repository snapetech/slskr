---
category: fixed
audience: users, operators
area: transfer-runtime
action: none
breaking: false
---

File-transfer uploads now use bounded transport-sized chunks, including when
resuming at end-of-file or using obfuscated connections.
