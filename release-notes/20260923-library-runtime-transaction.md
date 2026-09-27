---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Local manual imports now commit their library item and import counter in one SQLite transaction without replacing unrelated runtime statistics. If persistence fails, the failed item and counter are rolled back while concurrent updates to other state are preserved.
