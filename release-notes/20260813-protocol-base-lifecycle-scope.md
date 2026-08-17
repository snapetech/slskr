---
category: changed
audience: operators
area: protocol-parity
action: none
breaking: false
---
Protocol parity validation now scopes timeout, cancellation, reconnect, and failure evidence to the connection/session implementations that own that lifecycle; the frozen base wire-code declaration remains inventory evidence only.
