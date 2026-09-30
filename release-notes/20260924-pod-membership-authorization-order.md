---
category: changed
audience: users, operators
area: room-and-pod-membership
action: none
breaking: false
---
Join and leave requests now validate membership and roles while updating
pending state under one room-then-workflow lock order. Acceptors are checked
again after lock waits, so revoked permissions cannot consume pending requests.
