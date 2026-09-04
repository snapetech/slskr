---
category: fixed
audience: users, operators
area: security
action: none
breaking: false
---

SongID local-file checks and destination validation now fail closed when
filesystem path resolution cannot be completed, including paths traversing
existing symlinked ancestors. Legacy transfer deletion routes also report
file-removal failures instead of returning a successful response. Public
relay resolution now rejects reserved and IPv4-mapped private addresses.
