---
category: security
audience: operators
area: relay
action: none
breaking: false
---
Relay-agent download staging files are now created with owner-only permissions, preventing partially received remote content from being exposed through the local process umask before the final rename.
