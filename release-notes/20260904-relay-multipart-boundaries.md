---
category: fixed
audience: operators
area: relay
action: none
breaking: false
---
Relay multipart uploads now recognize only valid line-delimited boundaries, so
binary payloads containing boundary-like bytes are preserved instead of being
misparsed as truncated uploads.
