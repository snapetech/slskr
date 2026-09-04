---
category: security
audience: operators
area: relay
action: none
breaking: false
---

Relay agents now resolve the configured controller once per connection attempt and pin that address set for both HTTP and websocket traffic while retaining the configured hostname for TLS verification.
