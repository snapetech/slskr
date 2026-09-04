---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---
The runtime dashboards now distinguish an unavailable optional endpoint from daemon outages, authentication failures, and network errors instead of displaying fabricated empty state. The Web package no longer advertises stale audit commands for checker files that are not part of this repository, and its live remediation command points to the repository's actual script directory.
