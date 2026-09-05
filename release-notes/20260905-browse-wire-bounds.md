---
category: security
audience: operators
area: browse
action: none
breaking: false
---
Browse response parsers now cap wire sections, folders, and file records before iterating them, preventing malformed compressed peer responses from consuming excessive CPU while yielding no usable entries.
