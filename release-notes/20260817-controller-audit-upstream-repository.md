---
category: fixed
audience: operators
area: parity-evidence
action: none
breaking: false
---

Controller-route evidence now honors the explicitly supplied upstream Git
repository when materializing pinned slskd and slskdN source snapshots, so a
detached frozen worktree cannot be mistaken for the history-bearing oracle.
