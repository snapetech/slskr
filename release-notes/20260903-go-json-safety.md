---
category: security
audience: operators
area: client-libraries
action: none
breaking: false
---

The Go client now decodes API responses through explicit JSON container types, preventing unsafe arbitrary-interface deserialization while retaining object and array response compatibility.
