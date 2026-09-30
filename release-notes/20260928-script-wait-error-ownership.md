---
category: fixed
audience: operators
area: integration-scripts
action: none
breaking: false
---
Integration script wait errors retain cleanup ownership of the process group. Successful child reaping disarms that ownership before later cleanup, preserving the protection against targeting a reused process-group identifier.
