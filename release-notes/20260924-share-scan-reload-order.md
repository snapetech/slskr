---
category: changed
audience: users
area: file-sharing
action: none
breaking: false
---
If share settings change during an index rebuild, the daemon cancels the active
scan and keeps the new settings marked pending. A scan built from older
settings cannot replace the live or persisted share index.
