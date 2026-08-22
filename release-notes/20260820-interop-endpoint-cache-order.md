---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

The slskdN cross-client interop runner now executes target-initiated browse,
message, and transfer checks before its bounded 60-second injected endpoint
cache expires. This prevents the runner from falling back to a public peer
endpoint and misclassifying local replacement routing as unavailable.
