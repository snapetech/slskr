---
category: fixed
audience: operators
area: web
action: none
breaking: false
---
Repeated library-browser and native stream lookups now reuse local SHA-256 hashes until the file changes, avoiding repeated full-file reads for the same content.
