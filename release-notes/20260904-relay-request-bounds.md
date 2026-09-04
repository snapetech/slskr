---
category: security
audience: operators
area: relay
action: none
breaking: false
---
Relay transfer workflows now cap pending request state, preventing repeated authenticated operations from growing controller memory without a bound while expired requests are awaiting cleanup.
