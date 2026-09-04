---
category: fixed
audience: users
area: client-sdk
action: none
breaking: false
---
The TypeScript client now treats an explicit `timeout: 0` as disabling the
request deadline instead of aborting requests immediately.
