---
category: fixed
audience: operators
area: release-tooling
action: none
breaking: false
---
Release guard scripts now use the indexed Bash source path, removing ShellCheck SC2128 warnings and keeping the release gate clean under its configured shell lint policy.
