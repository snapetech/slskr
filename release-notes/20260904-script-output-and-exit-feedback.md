---
category: fixed
audience: operators
area: script-integrations
action: none
breaking: false
---

Script integrations now cap captured stdout and stderr at 1 MiB and report
non-zero exits even when the process emits no diagnostic output.
