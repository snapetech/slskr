---
category: security
audience: users, operators
area: protocol-limits
action: none
breaking: false
---
Client frame readers, writers, raw payloads, and compressed share decoding now cap caller-provided limits at the repository safety ceilings while preserving smaller protocol-specific limits.
