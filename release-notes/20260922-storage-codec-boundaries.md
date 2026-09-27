---
category: changed
audience: operators
area: share-codecs
action: none
breaking: false
---

Share-list parsing now reuses one bounded file-entry decoder and cache writers append escaped fields into a single buffer, reducing duplicate work while preserving wire and cache formats.
