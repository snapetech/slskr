# slskr Release Runbook

This is the release-prep path for binary archives. `slskr` is a single Rust
binary plus runtime web assets from `web/build`.

## Local Gate

```sh
scripts/run-release-gate.sh
```

This runs public-posture checks, shell syntax checks, Rust formatting, clippy,
workspace tests, RustSec audit when `cargo-audit` is installed, workspace
packaging, web tests, web build verification, and subpath smoke checks.

The live slskd automation-client compatibility smoke is opt-in because it starts
a local daemon and may install the Python `slskd-api` package:

```sh
SLSKR_RUN_SLSKD_API_COMPAT_SMOKE=1 scripts/run-release-gate.sh
```

## Local Archive

Build the host archive:

```sh
scripts/build-release-archive.sh --version dev-local
scripts/verify-release-artifacts.sh target/dist
```

The archive includes:

- `slskr` or `slskr.exe`
- `web/build`
- `README.md`, `LICENSE`, `NOTICE`, `COMPLIANCE.md`
- `docs/slskr.config.example.toml`
- `RUN.txt`

## CI Matrix

The release workflow builds these native runner variants:

- `linux-x64` on `ubuntu-latest`
- `linux-musl-x64` on `ubuntu-latest`
- `linux-arm64` on `ubuntu-24.04-arm`
- `macos-x64` on `macos-13-large`
- `macos-arm64` on `macos-14`
- `windows-x64` on `windows-latest`

Trigger it manually with `workflow_dispatch`, or push a tag named:

```text
release-v<semver>
```

Tag-triggered releases intentionally use the `release-v<semver>` convention,
for example `release-v1.2.3` or `release-v1.2.3-rc.1`. Plain `v*` tags and
loose `release-*` tags are not release triggers.

For a tag build, the workflow creates a GitHub Release and uploads all archives
plus `SHA256SUMS.txt`, `slskr-cyclonedx.json`, and
`slskr-dependency-manifest.json`. The JSON manifests are included in the
release checksum file and build-provenance attestation subjects.

The internal/unpublished Cargo crates intentionally remain at `0.0.0`. Binary
and archive version metadata comes from the release workflow: tag builds derive
the artifact version from `release-v<semver>`, while manual builds use the
`SLSKR_RELEASE_VERSION` value passed to `scripts/build-release-archive.sh`.
Archive roots are named `slskr-<version>-<target>` so the published package
version remains tied to the release input even though the workspace crates are
not published independently.
