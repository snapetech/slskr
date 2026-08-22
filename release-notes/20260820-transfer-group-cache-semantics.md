---
category: fixed
audience: operators
area: transfer-groups
action: none
breaking: false
---
Compatibility user-group lookups now preserve the frozen cache semantics:
unknown usernames remain in the default group until user information is
cached, while cached blacklisted users retain blacklist precedence.
