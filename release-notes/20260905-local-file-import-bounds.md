---
category: fixed
audience: users, operators
area: local-file-imports
action: none
breaking: false
---
Browser CSV, listening-history, and RustyMilk imports now bound local text reads before parsing, preventing oversized selected files from consuming unbounded browser memory.
