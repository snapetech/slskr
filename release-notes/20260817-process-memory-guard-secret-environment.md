---
category: security
audience: operators
area: process-memory-guard
action: none
breaking: false
---

Guarded systemd process environments are now supplied through a mode-restricted
temporary environment file instead of exposing caller environment values in the
transient unit command line. Required credentials and test settings remain
available to the guarded process without weakening the memory limit.
