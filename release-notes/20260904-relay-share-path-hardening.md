---
category: security
audience: operators
area: relay
action: none
breaking: false
---
Relay share database validation now rejects symlink and non-file paths before SQLite opens them, preventing restart-time ingestion from following an unexpected filesystem target.
