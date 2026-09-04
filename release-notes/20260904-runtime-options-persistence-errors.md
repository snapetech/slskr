---
category: fixed
audience: users, operators
area: compatibility-api
action: none
breaking: false
---

Compatibility mutations now report persistence failures, asynchronous share
rescans publish failure events, and persisted SongID jobs transition to a
visible failed state when recovery cannot queue them. Outbound peer dialing
also preserves the upstream regular-first fallback contract.
