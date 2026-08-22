---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
Windows Rust builds now keep the top-level memory guard while clearing the Unix-only compiler-wrapper setting that Cargo cannot execute on native Windows runners.
