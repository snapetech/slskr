---
category: fixed
audience: users, operators
area: http-head
action: none
breaking: false
---
HTTP HEAD requests for fallback pages and API responses now advertise the same representation length as GET while correctly omitting response bytes, improving client and proxy compatibility.
