---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Share-group and membership mutations now persist in their in-memory order.
Snapshots cannot overwrite a later group deletion, and a member request queued
behind deletion observes the removed group. Database waits no longer hold the
share-group write lock, so reads remain available while mutations wait.
