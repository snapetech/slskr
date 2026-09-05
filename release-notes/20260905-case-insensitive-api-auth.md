---
category: security
audience: users
area: http-authentication
action: none
breaking: false
---
HTTP API authentication now treats the `Bearer` and `ApiKey` authorization schemes case-insensitively across authorization, CSRF handling, session bootstrap, JWT revocation, and authenticated rate-limit partitioning.
