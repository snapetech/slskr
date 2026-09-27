---
category: changed
audience: operators
area: http-server
action: none
breaking: false
---
HTTP, HTTPS, and Unix-socket HTTP listeners now share a 256-connection limit.
Excess connections are closed before request work starts. HTTPS and Unix
handlers are owned by the managed shutdown registry and are canceled and joined
when the daemon stops.
