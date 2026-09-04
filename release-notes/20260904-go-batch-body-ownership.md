---
category: fixed
audience: users
area: go-sdk
action: none
breaking: false
---

Go batch builders now defensively copy typed slices in arbitrary JSON bodies,
including common values such as `[]string`, before retaining or returning an
operation.
