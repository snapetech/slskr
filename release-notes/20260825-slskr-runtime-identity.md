---
category: changed
audience: users, operators
area: runtime-identity
action: Reauthenticate native web sessions after upgrading, and use `SLSKR_CONTROLLER_PROFILE=native` or `legacy` when selecting a profile explicitly.
breaking: true
---

Native launches now use slskr identity in generated controller defaults, JWT claims, UI state, browser storage, and runtime profile output while legacy launches retain their frozen compatibility identity; native application state also keeps the initial update-availability field without changing the legacy response shape.
