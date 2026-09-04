---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---

Dependency audit gates now use a validated local npm audit cache when the registry audit endpoint is temporarily unavailable, while still failing on reported vulnerabilities.
