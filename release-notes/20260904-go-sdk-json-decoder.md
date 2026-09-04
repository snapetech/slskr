---
category: fixed
audience: operators
area: go-sdk-security
action: none
breaking: false
---

The Go SDK now decodes API JSON through concrete JSON representations before converting it to its dynamic response model, avoiding unsafe generic deserialization while preserving existing response shapes.
