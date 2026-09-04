---
category: security
audience: users, operators
area: jwt-revocation
action: none
breaking: false
---

JWT revocation state is now validated at startup and revocations report a
service failure when durable storage cannot be updated, preventing silent
revocation loss across restarts.
