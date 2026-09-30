---
category: fixed
audience: operators
area: parity-audits
action: none
breaking: false
---
Parity audits now follow the extracted Rust configuration and backfill owners. Configuration inventory includes nested implementation modules and excludes named test suites. Live backfill promotion still requires the remote route, transfer-token and hash assertions, and now verifies that the backfill owner is registered in the daemon.
