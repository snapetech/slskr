---
category: fixed
audience: operators
area: runtime-lifecycle
action: none
breaking: false
---
On Unix, integration scripts run in their own process group. Timeout, output-limit failure, and cancellation kill that group while its leader remains owned, preventing shell children from continuing after the script worker stops. Successful completion disarms the group guard immediately after waiting for the leader.
