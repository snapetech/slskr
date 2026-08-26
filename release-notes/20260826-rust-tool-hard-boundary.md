---
category: security
audience: operators
area: build-and-test-memory
action: Run `scripts/install-rust-tool-shims.sh` once on each workstation that runs local Rust commands.
breaking: false
---
Repository Rust commands now route through hard memory guards; dangerous `cargo fmt` and `rustfmt --check` modes are rejected, while formatting uses a per-file bounded emit-and-compare path.
