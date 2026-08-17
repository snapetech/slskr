---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

The parity manifest now records frozen persistence domains whose source
contracts provide durable append/upsert and read operations but no delete
operation. Their composite update/delete lifecycle cases remain visible and
are marked not applicable instead of requiring behavior absent from the
frozen targets.
