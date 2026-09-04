---
category: fixed
audience: operators
area: ci
action: none
breaking: false
---

The Rust formatting gate now consistently excludes the known historical monolithic web controller from incremental formatting checks, preventing pre-existing formatting debt from failing unrelated CI changes.
