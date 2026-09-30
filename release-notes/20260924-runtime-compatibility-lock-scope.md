---
category: changed
audience: operators
area: runtime-state
action: none
breaking: false
---
Runtime compatibility updates release runtime and relay locks before waiting
for SQLite. On persistence failure, rollback preserves unrelated changes.
Restart-request and bridge-running flags stay process-local, so later durable
updates cannot resurrect stale restart status. Bridge status turns true only
after its listener binds.
