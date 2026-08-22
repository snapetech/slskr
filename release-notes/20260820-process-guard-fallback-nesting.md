---
category: fixed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

The portable process-memory guard now marks its fallback shell as an active
guarded parent, matching the systemd path. Nested guarded Rust commands can
therefore use the same bounded opt-in behavior on systems without a user
systemd manager.
