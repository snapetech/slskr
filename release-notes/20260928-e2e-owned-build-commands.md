---
category: fixed
audience: operators
area: testing
action: none
breaking: false
---

Native E2E Cargo/npm commands now have a ten-minute deadline and cancellation
ownership. Stopping the last waiting node cancels its command tree and joins
child close; other waiting nodes keep shared builds running. Completed builds
can rebuild after source changes.
