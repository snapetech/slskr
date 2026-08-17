---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

Interrupting a systemd-backed parity or frontend command now stops its transient memory-guard unit as well, preventing an abandoned browser process from surviving the command that launched it.
