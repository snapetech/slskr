---
category: changed
audience: operators
area: overlay-transport
action: none
breaking: false
---
Added the `overlay_data.max_concurrent_streams` setting, defaulting to eight inbound streams, so QUIC data listeners bound stream concurrency before accepting payload work.
