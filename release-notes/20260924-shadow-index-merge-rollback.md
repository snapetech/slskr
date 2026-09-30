---
category: fixed
audience: operators
area: persistence
action: none
breaking: false
---
Shadow-index sync now restores its shadow-record write when saving companion realm indexes fails and reports storage unavailability so clients can retry.
