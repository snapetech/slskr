---
category: fixed
audience: users, operators
area: sdk-peer-connection-management
action: none
breaking: false
---
Managed peer connection requests now apply their deadline while waiting for a per-peer connection slot, so a stalled connection cannot make later requests wait indefinitely.
