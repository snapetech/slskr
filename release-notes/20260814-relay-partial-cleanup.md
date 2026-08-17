---
category: changed
audience: operators
area: relay-downloads
action: none
breaking: false
---

Relay-agent downloads now remove their temporary partial file when streaming,
flush, or final replacement fails. Completed files and existing destinations
retain the prior atomic replacement behavior.
