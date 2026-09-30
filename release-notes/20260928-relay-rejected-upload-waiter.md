---
category: fixed
audience: users
area: relay
action: none
breaking: false
---
Relay file uploads rejected for authorization or a mismatched filename now notify the waiting stream immediately when their one-use token is consumed. Rejected tokens remain unusable, and stalled stream waiters no longer wait for a timeout on those failures.
