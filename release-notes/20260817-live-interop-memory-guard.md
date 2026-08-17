---
category: changed
audience: operators
area: live-interop-safety
action: none
breaking: false
---

The live interop launcher now applies the repository's bounded virtual-memory
limit to reused daemon binaries as well as build-time commands, preventing a
prebuilt test run from bypassing the memory guard.
