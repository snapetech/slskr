---
category: fixed
audience: operators
area: transfers
action: none
breaking: false
---
Legacy transfer-array requests now resolve destination paths before taking the transfer write lock, keeping concurrent transfer reads and updates responsive.
