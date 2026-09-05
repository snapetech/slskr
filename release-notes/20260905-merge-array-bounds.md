---
category: security
audience: operators
area: mesh-sync-api
action: none
breaking: false
---
HashDb, shadow-index, realm-index, and legacy mesh sync handlers now reject oversized arrays before cloning and deserializing batches beyond their store limits, reducing avoidable memory and CPU pressure from oversized synchronization requests.
