---
category: fixed
audience: operators
area: release-validation
action: none
breaking: false
---
Chocolatey release verification now accepts the root-relative ./filename entries emitted in published SHA256SUMS files. Exact filenames, a unique entry, and the downloaded archive hash remain required. Duplicate aliases, other directories, extra suffixes, missing entries, and incorrect hashes are rejected.
