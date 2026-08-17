---
category: changed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

Workspace Rust tests now use the bounded focused slskR test target by default,
with test debug metadata, codegen, and LTO constrained by the repository build
guard. The historical monolithic controller suite is opt-in.
