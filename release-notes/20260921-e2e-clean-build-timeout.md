---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---

The serialized browser acceptance suite now allows a cold optimized daemon build to finish before starting real-node tests, while retaining single-worker execution for local fixture isolation.
