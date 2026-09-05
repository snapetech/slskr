---
category: fixed
audience: users
area: sdk
action: none
breaking: false
---
The Go SDK now preserves request cancellation and deadline errors while reading failed responses, so a stalled error body is not mislabeled as an API error.
