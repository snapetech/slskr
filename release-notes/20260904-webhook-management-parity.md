---
category: changed
audience: users, operators
area: webhook-management
action: none
breaking: false
---

The admin webhook listing now exposes the same delivery retry and timeout
fields as the regular webhook listing, and event dispatch no longer holds the
webhook configuration lock while waiting for audit persistence.
