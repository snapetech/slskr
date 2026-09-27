---
category: changed
audience: operators
area: transfers
action: none
breaking: false
---
Oversized transfer-event logs now rotate before the active file header is
validated. The previous log remains as `transfer.events.tsv.old`, and appends
continue in a fresh v2 event file.
