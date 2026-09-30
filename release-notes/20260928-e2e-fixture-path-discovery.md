---
category: fixed
audience: operators
area: test-fixtures
action: none
breaking: false
---
The E2E harness now finds the Cargo checkout from the repository root, web directory, or nested test directories. Optional media checks require nonempty regular files. Fixture fetching passes its script path as an argument rather than interpolating a shell command.
