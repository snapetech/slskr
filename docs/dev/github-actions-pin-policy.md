# GitHub Actions Pin Policy

slskR workflows pin external actions to reviewed 40-character commit SHAs. Version
tags and branches are tracked here only as review context so updates happen through
explicit code review instead of mutable workflow dependency drift.

Run `scripts/check-workflow-release-policy.sh` after changing `.github/workflows`.
The gate fails if any external `uses:` reference is not pinned to a full commit SHA
or if a pinned action is missing from this ledger.

| Action | Reviewed ref | Pinned commit | Notes |
| --- | --- | --- | --- |
| `actions/checkout` | `v6` | `3d3c42e5aac5ba805825da76410c181273ba90b1` | CI, release, live parity, local identity, and CodeQL checkout. |
| `actions/cache` | `v6.1.0` | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` | Cached pinned `cargo-audit` binaries for CI and release gates. |
| `dtolnay/rust-toolchain` | `stable` | `29eef336d9b2848a0b548edc03f92a220660cdb8` | Rust toolchain install for CI, release, and live parity jobs. |
| `Swatinem/rust-cache` | `v2` | `e18b497796c12c097a38f9edb9d0641fb99eee32` | Dereferenced tag target for Rust cache setup. |
| `actions/setup-node` | `v6` | `820762786026740c76f36085b0efc47a31fe5020` | Node setup for web, dashboard, TypeScript SDK, and live parity gates. |
| `actions/setup-go` | `v7.0.0` | `b7ad1dad31e06c5925ef5d2fc7ad053ef454303e` | Go SDK test setup. |
| `actions/setup-python` | `v7.0.0` | `5fda3b95a4ea91299a34e894583c3862153e4b97` | Python SDK and slskd API compatibility smoke setup. |
| `actions/upload-artifact` | `v7.0.1` | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Release archive and live parity artifact upload. |
| `actions/download-artifact` | `v8.0.1` | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` | Release archive download before publishing. |
| `actions/attest-build-provenance` | `v4` | `0f67c3f4856b2e3261c31976d6725780e5e4c373` | Dereferenced tag target for release asset attestations. |
| `softprops/action-gh-release` | `v3` | `3d0d9888cb7fd7b750713d6e236d1fcb99157228` | GitHub Release publisher. |
| `docker/setup-qemu-action` | `v4.2.0` | `96fe6ef7f33517b61c61be40b68a1882f3264fb8` | QEMU setup for multi-architecture Docker release images. |
| `docker/setup-buildx-action` | `v4.2.0` | `bb05f3f5519dd87d3ba754cc423b652a5edd6d2c` | Docker Buildx setup for multi-architecture release images. |
| `docker/login-action` | `v4.5.1` | `abd2ef45e78c5afb21d64d4ca52ee8550d9572c7` | GHCR and Docker Hub authentication for release images. |
| `docker/build-push-action` | `v7` | `53b7df96c91f9c12dcc8a07bcb9ccacbed38856a` | Multi-architecture Docker release image build and push. |
| `github/codeql-action/init` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL initialization for GitHub code scanning. |
| `github/codeql-action/autobuild` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL autobuild for analyzable language matrix entries. |
| `github/codeql-action/analyze` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL SARIF upload and alert generation. |

To update an action, resolve the new trusted ref, replace the workflow SHA and the
matching ledger row in the same change, and run the remediation baseline.
