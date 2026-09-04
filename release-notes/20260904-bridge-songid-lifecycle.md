---
category: fixed
audience: users, operators
area: runtime-lifecycle
action: none
breaking: false
---

Embedded bridge connections now have bounded I/O and terminate cleanly when
their listener fails. SongID jobs also expose failed processing states instead
of remaining indefinitely active after worker or persistence errors.
