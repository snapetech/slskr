---
category: fixed
audience: users
area: transfers
action: none
breaking: false
---
When transfer persistence fails, rollback now restores only unchanged rows and preserves newer concurrent transfer updates. Failed staged transfers are removed only when they still match the staged version.
