---
category: fixed
audience: operators
area: retention
action: none
breaking: false
---

File-retention scans now stop traversing after their owning background task is cancelled during shutdown, once any active filesystem call returns.
