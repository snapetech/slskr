---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---

Shared-stream windows opened by the Rust/WASM interface now use an opener-isolated, referrer-free policy and report popup-blocked or browser errors instead of claiming success unconditionally.
