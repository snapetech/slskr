---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

Parity validation now exercises the durable share-file projection through
create/read, replacement and stale-row removal, restart rehydration,
transactional concurrent snapshots, and corrupt-row failure handling. Derived
directory, filename-index, and scan-history contracts remain separately open.
