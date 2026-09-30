---
category: fixed
audience: operators
area: diagnostic-tools
action: none
breaking: false
---
CLI peer, server, and transfer probes now own their fixture and accept tasks. Probe timeout, early failure, and parent cancellation abort pending child tasks, allowing their temporary listeners and sockets to close instead of continuing after the probe exits.
