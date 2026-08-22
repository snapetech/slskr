---
category: fixed
audience: operators
area: parity-certification
action: none
breaking: false
---

Production smoke, certification, interop, and daemon-launch commands now select
the `slskr` binary explicitly after the bounded differential runner added a
second package binary. A static guard prevents future unqualified `cargo run`
calls from making those workflows ambiguous.
