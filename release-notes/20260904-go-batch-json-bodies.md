---
category: fixed
audience: users
area: go-sdk
action: upgrade
breaking: true
---

The Go batch SDK now accepts any JSON body shape supported by the batch API,
including arrays and scalar values. `BatchOperation.Body` is now represented as
an arbitrary JSON value instead of an object-only map. Go callers that index
`BatchOperation.Body` directly must update those accesses to use a type
assertion.
