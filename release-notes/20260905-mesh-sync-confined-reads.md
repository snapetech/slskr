---
category: security
audience: operators
area: mesh-sync
action: none
breaking: false
---
Mesh-sync content reads now use the configured share-root confinement and no-follow file opening path, preventing swapped path components from disclosing unrelated files.
