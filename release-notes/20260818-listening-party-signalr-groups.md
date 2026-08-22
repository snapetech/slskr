---
category: fixed
audience: users
area: web-compatibility
action: none
breaking: false
---

Listening-party SignalR connections now honor the frozen client's `JoinParty` and `LeaveParty` groups, so `partyState` updates are delivered only to the matching pod channel.
