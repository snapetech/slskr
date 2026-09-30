---
category: fixed
audience: operators
area: live-interop
action: none
breaking: false
---
CLI live-soak peer handlers and indirect probes now have bounded worker sets and joined cleanup. Stalled sockets close when their listener scope ends, the watchdog is joined, and failures cancel remaining work. A completed worker frees capacity before another is admitted. No listener port is added.
