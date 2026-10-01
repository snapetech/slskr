---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
Release security scans now exclude the separately checked-out parity repository, keeping findings from its vendored code and dependencies out of slskR's release results while continuing to scan slskR's source and dependency files.
