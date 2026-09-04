---
category: fixed
audience: operators
area: retention
action: none
breaking: false
---

Retention now deletes search records in one database transaction, reports
cleanup failures, and restores in-memory records when transfer deletion fails
without a concurrent transfer change.
