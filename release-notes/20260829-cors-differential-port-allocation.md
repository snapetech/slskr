---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
The CORS differential gate now allocates distinct listener ports and retries transient bind races, preventing nondeterministic release failures caused by an ephemeral port collision.
