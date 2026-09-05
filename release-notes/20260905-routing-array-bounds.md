---
category: fixed
audience: users, operators
area: controller-api
action: none
breaking: false
---
Library remediation, release-radar, PodCore routing, and PodCore discovery requests now reject oversized caller-supplied arrays before deduplication, state mutation, or peer fan-out.
