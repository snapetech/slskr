---
category: fixed
audience: users, operators
area: transfers
action: none
breaking: false
---

Transfer API mutations now reject missing or impossible progress, partial successful completions, and non-terminal statuses; cancelled transfers no longer emit a misleading completion webhook.
