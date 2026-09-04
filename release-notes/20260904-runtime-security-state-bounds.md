---
category: security
audience: users, operators
area: runtime-security
action: none
breaking: false
---
Mesh-sync violation tracking and failed-upload cooldowns now bound peer identifiers, reclaim expired state, and evict the oldest active entry at capacity so hostile peer churn cannot grow controller memory without disabling recovery controls.
