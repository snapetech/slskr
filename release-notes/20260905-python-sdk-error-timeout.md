---
category: fixed
audience: users
area: sdk
action: none
breaking: false
---
The Python SDK now preserves request timeouts while reading error response bodies, so a stalled failed response is reported as a timeout instead of being misclassified as an API error.
