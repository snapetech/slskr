---
category: fixed
audience: users, operators
area: security
action: none
breaking: false
---

Relay-agent uploads now open shared files through the same confined path
validation used by direct peer uploads, closing a symlink-swap escape between
share lookup and file streaming.
