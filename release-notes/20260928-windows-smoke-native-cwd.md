---
category: fixed
audience: operators
area: ci-validation
action: none
breaking: false
---
Windows Smoke now compares native working-directory paths and uses the same pinned Rust binary and full Perl selection as the main platform matrix. CI runs its installed workflow linter. Policy checks require the shared Perl helper and actual workflow lint execution.
