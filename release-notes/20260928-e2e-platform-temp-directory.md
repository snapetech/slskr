---
category: fixed
audience: operators
area: e2e-harness
action: none
breaking: false
---
The peer test harness now creates daemon state in the operating system's temporary directory. Failed-startup cleanup no longer depends on a Unix-only /tmp path when the harness runs on Windows.
