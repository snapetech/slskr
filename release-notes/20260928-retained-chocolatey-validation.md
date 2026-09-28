---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---
The manual Chocolatey workflow now accepts publish=false to validate and retain a package without pushing it. The existing publication default remains. Validated nupkg files, checksums, and source-bound receipts are retained for 30 days under a bounded job deadline. Release documentation describes this option and retained React browser audits.
