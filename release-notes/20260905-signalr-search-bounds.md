---
category: fixed
audience: users, operators
area: signalr-search
action: none
breaking: false
---
Large search histories and result updates now stay within the SignalR message ceiling by falling back to the same metadata needed for REST hydration instead of disconnecting the hub.
