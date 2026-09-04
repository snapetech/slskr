---
category: changed
audience: users, operators
area: protocol-parity
action: none
breaking: false
---

Room-join decoding now builds each user record once, reducing peak memory while handling large valid room responses.
