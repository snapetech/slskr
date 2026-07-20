# GitHub Actions Pin Policy

slskR workflows pin external actions to reviewed 40-character commit SHAs. Version
tags and branches are tracked here only as review context so updates happen through
explicit code review instead of mutable workflow dependency drift.

Run `scripts/check-workflow-release-policy.sh` after changing `.github/workflows`.
The gate fails if any external `uses:` reference is not pinned to a full commit SHA
or if a pinned action is missing from this ledger.

| Action | Reviewed ref | Pinned commit | Notes |
| --- | --- | --- | --- |
| `actions/checkout` | `v6` | `9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0` | CI, release, live parity, local identity, and CodeQL checkout. |
| `actions/cache` | `v6.1.0` | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` | Cached pinned `cargo-audit` binaries for CI and release gates. |
| `dtolnay/rust-toolchain` | `stable` | `29eef336d9b2848a0b548edc03f92a220660cdb8` | Rust toolchain install for CI, release, and live parity jobs. |
| `Swatinem/rust-cache` | `v2` | `e18b497796c12c097a38f9edb9d0641fb99eee32` | Dereferenced tag target for Rust cache setup. |
| `actions/setup-node` | `v6` | `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e` | Node setup for web, dashboard, TypeScript SDK, and live parity gates. |
| `actions/setup-go` | `v6` | `924ae3a1cded613372ab5595356fb5720e22ba16` | Go SDK test setup. |
| `actions/setup-python` | `v6` | `ece7cb06caefa5fff74198d8649806c4678c61a1` | Python SDK and slskd API compatibility smoke setup. |
| `actions/upload-artifact` | `v7.0.1` | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Release archive and live parity artifact upload. |
| `actions/download-artifact` | `v8.0.1` | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` | Release archive download before publishing. |
| `actions/attest-build-provenance` | `v4` | `0f67c3f4856b2e3261c31976d6725780e5e4c373` | Dereferenced tag target for release asset attestations. |
| `softprops/action-gh-release` | `v3` | `718ea10b132b3b2eba29c1007bb80653f286566b` | GitHub Release publisher. |
| `docker/setup-qemu-action` | `v4.2.0` | `96fe6ef7f33517b61c61be40b68a1882f3264fb8` | QEMU setup for multi-architecture Docker release images. |
| `docker/setup-buildx-action` | `v4.2.0` | `bb05f3f5519dd87d3ba754cc423b652a5edd6d2c` | Docker Buildx setup for multi-architecture release images. |
| `docker/login-action` | `v4.4.0` | `af1e73f918a031802d376d3c8bbc3fe56130a9b0` | GHCR and Docker Hub authentication for release images. |
| `docker/build-push-action` | `v7` | `53b7df96c91f9c12dcc8a07bcb9ccacbed38856a` | Multi-architecture Docker release image build and push. |
| `github/codeql-action/init` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL initialization for GitHub code scanning. |
| `github/codeql-action/autobuild` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL autobuild for analyzable language matrix entries. |
| `github/codeql-action/analyze` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL SARIF upload and alert generation. |

To update an action, resolve the new trusted ref, replace the workflow SHA and the
matching ledger row in the same change, and run the remediation baseline.
