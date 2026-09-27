---
category: changed
audience: operators
area: transfers
action: none
breaking: false
---
Transfer queue snapshots now carry per-transfer revisions through SQLite. Older
delayed writes cannot replace newer progress or recreate a deleted transfer;
transfer event history remains ordered for rapid transitions. Existing
databases migrate automatically at startup.
