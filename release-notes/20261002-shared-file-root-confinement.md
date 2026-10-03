---
category: security
audience: operators
area: share-streaming
action: none
breaking: false
---

Shared-file streaming now revalidates that an opened path resolves inside a configured share root. Platforms without Unix descriptor-relative opens also reject symlink and reparse-point components when share symlink following is disabled.
