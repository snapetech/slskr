---
category: fixed
audience: users, operators
area: overlay-lifecycle
action: none
breaking: false
---

Overlay tunnel shutdown now waits for forwarding tasks, reports close timeouts,
preserves verified swarm publication errors, serializes relay-share manifest
updates, validates and durably publishes relay downloads, and logs failed
one-way pod-message or QUIC relay dispatches instead of silently treating
cleanup or delivery as successful.
