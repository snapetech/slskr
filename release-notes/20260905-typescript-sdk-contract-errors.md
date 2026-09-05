---
category: fixed
audience: users, operators
area: client-sdk
action: none
breaking: false
---
The TypeScript client now reports malformed successful API and batch envelopes as contract errors instead of silently returning empty collections or default-valued records; valid legacy collection aliases remain supported.
