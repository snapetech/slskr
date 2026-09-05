---
category: security
audience: users, operators
area: file-transfer
action: none
breaking: false
---
File-transfer chunk reads now cap explicit caller-provided limits at the existing 16 MiB safety ceiling, preventing a larger requested limit from turning a peer-controlled read into an unbounded allocation.
