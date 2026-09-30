---
category: changed
audience: operators
area: continuous-integration
action: none
breaking: false
---
The main CI gate retains reproducibility evidence for 30 days and links it from the job summary. Metadata records the source commit, run, observed tool versions, lock/config hashes, coverage summary, and built Web entry point. Missing or inconsistent provenance fails validation; unknown worktree state is reported explicitly.
