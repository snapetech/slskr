---
category: security
audience: operators
area: outbound
action: none
breaking: false
---
Outbound integrations that validate and pin DNS results now bypass ambient HTTP proxy settings, preventing proxy routing from circumventing the repository’s private-address and SSRF protections.
