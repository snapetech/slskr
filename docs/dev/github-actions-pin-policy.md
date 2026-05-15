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
| `github/codeql-action/init` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL initialization for GitHub code scanning. |
| `github/codeql-action/autobuild` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL autobuild for analyzable language matrix entries. |
| `github/codeql-action/analyze` | `v4` | `5e316336eb4f107009e477d4bfbfff13d7250fae` | CodeQL SARIF upload and alert generation. |

To update an action, resolve the new trusted ref, replace the workflow SHA and the
matching ledger row in the same change, and run the remediation baseline.
