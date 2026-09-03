---
category: security
audience: users, operators
area: runtime
action: none
breaking: false
---
The Rust dependency lockfile now uses chacha20 0.10.2 instead of the yanked 0.10.0 release, removing the stale dependency warning without changing the runtime API.
