---
category: security
audience: operators
area: release-pipeline
action: none
breaking: false
---
Release assembly now passes repository and ref metadata through step environment variables instead of shell-interpolating GitHub context, while lifecycle probes validate HTTP endpoints and preserve private test-directory permissions.
