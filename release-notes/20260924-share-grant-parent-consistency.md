---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Share grant creation now rechecks that its collection still exists after
waiting for the grant store, and the database rejects writes for deleted
collections. A concurrent collection deletion can no longer leave an orphaned
share grant.
