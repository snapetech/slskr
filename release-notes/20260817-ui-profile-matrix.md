---
category: changed
audience: operators
area: parity-certification
action: none
breaking: false
---

The frozen-target UI comparator now requires separate live slskR `slskd` and
`slskdn` compatibility-profile backends and compares each profile with its
matching frozen target. A single replacement backend or stubbed event feed
cannot satisfy strict UI parity evidence.
