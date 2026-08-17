---
category: fixed
audience: operators
area: interop-evidence
action: none
breaking: false
---

Cross-client interop runners now load the generated account pool consistently and fail closed when any recorded check is non-OK, preserving failed TSV evidence instead of reporting a false successful run.
