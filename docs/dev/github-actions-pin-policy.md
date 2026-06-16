# GitHub Actions Pin Policy

slskR workflows pin external actions to reviewed 40-character commit SHAs. Version
tags and branches are tracked here only as review context so updates happen through
explicit code review instead of mutable workflow dependency drift.

Run `scripts/check-workflow-release-policy.sh` after changing `.github/workflows`.
The gate fails if any external `uses:` reference is not pinned to a full commit SHA
or if a pinned action is missing from this ledger.

| Action | Reviewed ref | Pinned commit | Notes |
| --- | --- | --- | --- |
| `actions/checkout` | `v6` | `de0fac2e4500dabe0009e67214ff5f5447ce83dd` | CI, release, live parity, local identity, and CodeQL checkout. |
| `dtolnay/rust-toolchain` | `stable` | `29eef336d9b2848a0b548edc03f92a220660cdb8` | Rust toolchain install for CI, release, and live parity jobs. |
| `Swatinem/rust-cache` | `v2` | `e18b497796c12c097a38f9edb9d0641fb99eee32` | Dereferenced tag target for Rust cache setup. |
| `actions/setup-node` | `v6` | `48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e` | Node setup for web, dashboard, TypeScript SDK, and live parity gates. |
| `actions/setup-go` | `v6` | `4a3601121dd01d1626a1e23e37211e3254c1c06c` | Go SDK test setup. |
| `actions/setup-python` | `v6` | `a309ff8b426b58ec0e2a45f0f869d46889d02405` | Python SDK and slskd API compatibility smoke setup. |
| `actions/upload-artifact` | `v7.0.1` | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | Release archive and live parity artifact upload. |
| `actions/download-artifact` | `v8.0.1` | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` | Release archive download before publishing. |
| `actions/attest-build-provenance` | `v4` | `a2bbfa25375fe432b6a289bc6b6cd05ecd0c4c32` | Dereferenced tag target for release asset attestations. |
| `softprops/action-gh-release` | `v3` | `b4309332981a82ec1c5618f44dd2e27cc8bfbfda` | GitHub Release publisher. |
| `docker/setup-qemu-action` | `v4.1.0` | `06116385d9baf250c9f4dcb4858b16962ea869c3` | QEMU setup for multi-architecture Docker release images. |
| `docker/setup-buildx-action` | `v4.1.0` | `d7f5e7f509e45cec5c76c4d5afdd7de93d0b3df5` | Docker Buildx setup for multi-architecture release images. |
| `docker/login-action` | `v4.2.0` | `650006c6eb7dba73a995cc03b0b2d7f5ca915bee` | GHCR and Docker Hub authentication for release images. |
| `docker/build-push-action` | `v7` | `f9f3042f7e2789586610d6e8b85c8f03e5195baf` | Multi-architecture Docker release image build and push. |
| `github/codeql-action/init` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL initialization for GitHub code scanning. |
| `github/codeql-action/autobuild` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL autobuild for analyzable language matrix entries. |
| `github/codeql-action/analyze` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL SARIF upload and alert generation. |

To update an action, resolve the new trusted ref, replace the workflow SHA and the
matching ledger row in the same change, and run the remediation baseline.
