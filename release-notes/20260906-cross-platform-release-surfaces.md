---
category: fixed
audience: users, operators
area: release-pipeline
action: none
breaking: false
---
Linux ARM64 packages, Snap and Flatpak metadata, downstream release jobs, container identities, and deployment manifests now use the matching release assets and runtime ports across supported platforms. Release CI uses the current native Apple Silicon runner, and source Docker builds exclude generated local outputs from their build context.
