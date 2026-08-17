---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

The process-memory guard now preserves the repository working directory when it launches a systemd user unit, so guarded parity commands resolve relative scripts and fixtures exactly as they do outside the unit.
