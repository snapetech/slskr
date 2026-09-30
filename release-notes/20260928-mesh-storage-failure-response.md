---
category: fixed
audience: users
area: content-discovery
action: none
breaking: false
---
Hash database operations, including mesh merge and publish, return HTTP 503 with a sanitized storage-unavailable response when durable hash storage fails. Failed writes retain their existing rollback behavior instead of exposing storage details as a client-validation error.
