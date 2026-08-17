---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

The parity manifest now classifies HashDb, job-cache, Virtual Soulfind, and
migration-only FileSources persistence by their real source contracts. Append-
only stores mark only deletion as not applicable, while durable read/write
lifecycles retain their normal evidence requirements.
