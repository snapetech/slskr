---
category: security
audience: users, operators
area: regex-matching
action: none
breaking: false
---
Backtracking regular-expression matches now use bounded worker admission, preventing concurrent timeout-prone matches from creating unbounded operating-system threads and stacks.
