---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

Parity validation now distinguishes security data contracts, option-only
configuration types, timing declarations, and interface-only policy/token
contracts from concrete enforcement components. Those declarations are linked
to their owning controller, service, or transport evidence instead of creating
duplicate security-control obligations.
