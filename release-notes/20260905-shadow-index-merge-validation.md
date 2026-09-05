---
category: fixed
audience: users
area: controller-api
action: none
breaking: false
---
Combined shadow-index synchronization now validates all realm indexes before applying shadow records, so rejected realm-index payloads do not leave partial shadow state behind.
