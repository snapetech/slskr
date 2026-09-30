---
category: fixed
audience: operators
area: test-fixtures
action: Run scripts/fetch-test-fixtures.sh to install the optional playback fixtures.
breaking: false
---
Optional Sintel and Open Goldberg media now have pinned download sizes and hashes. Fetching validates HTTPS sources and existing caches, bounds reads and duration, and removes partial files on failure. Movie E2E tests require only their actual Sintel dependency.
