---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

The process-memory guard regression suite now verifies that systemd-backed commands retain the repository working directory, preventing bounded evidence commands from silently resolving the wrong files.
