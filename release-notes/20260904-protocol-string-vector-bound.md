---
category: security
audience: users, operators
area: protocol
action: none
breaking: false
---

Server string-vector decoding now rejects excessive item counts before allocating one string object per item, reducing memory amplification from malformed or hostile protocol frames.
