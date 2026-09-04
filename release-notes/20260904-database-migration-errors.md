---
category: fixed
audience: users, operators
area: persistence
action: none
breaking: false
---

Database startup migrations now surface unexpected message and share-schema errors instead of silently continuing; already-applied columns remain accepted for existing databases.
