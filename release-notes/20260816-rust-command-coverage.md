---
category: changed
audience: operators
area: build-safety
action: none
breaking: false
---
Build-safety checks now cover Cargo run, audit, metadata, tree, and benchmark entry points so repository Rust commands and their runtime children inherit the exclusive lock and memory ceiling.
