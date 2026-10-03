---
category: security
audience: operators
area: audit-tooling
action: none
breaking: false
---

Audit and policy scans now report matcher and file-read errors instead of treating them as clean results. The temporary account generator also rejects symlinked outputs and preserves existing entries when it cannot read the current file. The remediation gate checks that bug-ledger IDs remain unique and ordered.
