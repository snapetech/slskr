---
category: fixed
audience: operators
area: transfers
action: none
breaking: false
---
Per-user download enqueue routes now resolve destination paths before taking the transfer write lock, keeping concurrent transfer reads and updates responsive.
