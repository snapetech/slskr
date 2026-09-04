---
category: fixed
audience: operators
area: pods
action: none
breaking: false
---
Pod persistence now enforces its 8 MiB limit on writes as well as reads, rejects unsafe revocation identities and symlink markers, and bounds optional member metadata before storing it.
