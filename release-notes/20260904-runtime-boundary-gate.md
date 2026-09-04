---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
The runtime-boundary hardening gate now scans the shared networking utility module, so its NAT64 and special-use address checks are validated where they are implemented instead of producing a false CI failure.
