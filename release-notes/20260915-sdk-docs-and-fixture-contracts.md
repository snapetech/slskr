---
category: fixed
audience: users, operators
area: sdk-release-packaging
action: none
breaking: false
---

SDK examples and maintained integration documentation now use the current pagination, message, transfer-progress, and WebSocket event contracts. Chocolatey publishing verifies release checksums and embeds a tag-qualified asset URL, while E2E fixture validation rejects manifest size or SHA-256 drift.
