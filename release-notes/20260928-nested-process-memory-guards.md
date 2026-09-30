---
category: fixed
audience: operators
area: development-audits
action: none
breaking: false
---
Nested frontend and browser audit commands now reuse a verified memory boundary instead of adding a virtual-address limit inside an existing bounded cgroup. Linux nesting verifies the actual resident-memory and zero-swap limits; tighter nested memory requests still apply. This allows Node and Chromium address reservations without lifting the 4 GiB memory ceiling.
