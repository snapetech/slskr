---
category: security
audience: users, operators
area: relay
action: none
breaking: false
---
Relay upload parsing now bounds multipart headers, parameters, share metadata size, and metadata entry counts before allocating large request-derived structures.
