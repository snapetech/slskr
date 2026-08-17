---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

Failed cross-client interop runs now retain a row-level TSV artifact for diagnosis, and distributed-peer failures now make the runner fail, while the canonical all-green result remains unavailable until every row passes.
