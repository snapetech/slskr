# slskr Release Runbook

This is the release-prep path for binary archives. `slskr` is a single Rust
binary plus the production React/Vite Web UI assets staged as `web/build`.

## Local Gate

```sh
scripts/run-release-gate.sh
```

This runs public-posture checks, shell syntax checks, Rust formatting, clippy,
workspace tests, RustSec audit when `cargo-audit` is installed, workspace
packaging, web tests, Rust/WASM web checks, the Rust web UI headless parity
audit, and subpath smoke checks.

All compile-capable Cargo commands in this repository use the normal Rust
toolchain. The workspace config keeps the large daemon crate on one Cargo job,
turns off unnecessary dev debug metadata, and uses 16 codegen units with
ThinLTO for release builds:

```sh
cargo build --release -p slskr
cargo test --workspace
```

Rust formatting uses `scripts/check-rust-format.sh`, which checks changed source
files individually and compares emitted output locally rather than asking
rustfmt to construct the monolithic controller's full diff.

The `slskr` package defaults to its focused controller-test target. The
historical monolithic controller suite remains an explicit opt-in with
`--features full-controller-tests`; the default workspace test path keeps that
large proof module out of ordinary edit/test cycles.

Browser and frontend Node subprocesses in the parity and release gates use
`scripts/with-process-memory-guard.sh`; Rust commands do not. This keeps the
non-Rust browser tooling bounded without imposing its smaller process cap on
LLVM.

The live slskd automation-client compatibility smoke is opt-in because it starts
a local daemon and may install the Python `slskd-api` package:

```sh
SLSKR_RUN_SLSKD_API_COMPAT_SMOKE=1 scripts/run-release-gate.sh
```

CI also runs a lighter scheduled/manual `Live Parity` workflow. That workflow
executes the Rust web UI headless parity audit and the hermetic local
`slskd_api` automation compatibility smoke, then uploads the Rust UI screenshots,
web bundle, and daemon log as artifacts. The same workflow also has an optional
credentialed public-live job: when the `SLSKR_LIVE_INTEROP_ENV` repository secret
contains the same env-file variables used by `scripts/run-live-interop-matrix.sh`,
CI runs login, local peer, private-message, and room-message probes and uploads
`target/live-interop`; when the secret is absent, it uploads an explicit skipped
TSV artifact.

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
- `web/build` with `index.html`, hashed JavaScript/CSS assets, icons, and the
  web app manifest
- `README.md`, `LICENSE`, `NOTICE`, `COMPLIANCE.md`
- `docs/slskr.config.example.toml`
- `RUN.txt`

## Release notes and changelog

User-facing, security, operational, and user-facing documentation changes add
one new validated fragment under `release-notes/`. Pull-request validation
requires that fragment or an explicit internal-only selection, and fragments
are append-only. Each fragment records its category, audience, product area,
required action, and breaking-change status.

Preview the grouped release text locally with:

```sh
python3 scripts/release_notes.py preview --base <base> --head <head>
```

The tag workflow publishes a concise curated body as `release/RELEASE_NOTES.md`
and sends that same summary in the Discord embed. It also uploads the complete
machine-validated fragment assembly as `release/RELEASE_NOTES_FULL.md`, keeping
the release page readable without discarding detailed change history. The
pull-request job exposes a separate capture-metadata table so the source
fragment fields remain auditable. Keep `CHANGELOG.md` append-only at the
release-section level and move shipped `## [Unreleased]` bullets into a dated
section when preparing a release; the CI and tag workflows reject a missing or
placeholder section.

## CI and release matrix

The `CI` workflow runs the platform matrix on every pull request and push to
`main`. Each matrix job builds a target archive, verifies its checksum and
layout, and uploads the archive for seven days so a platform-specific build can
be downloaded for testing. Native workspace tests run on Linux x64, Linux
AArch64, both macOS architectures, and Windows; the additional musl jobs smoke
test their built static binary through the same archive verifier.

The release workflow uses the same native runner and target mapping:

| Target | CI and release runner | Rust target | CI exercise |
| --- | --- | --- | --- |
| `linux-x64` | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | Full Linux tests and archive smoke |
| `linux-musl-x64` | `ubuntu-latest` | `x86_64-unknown-linux-musl` | Static archive build and smoke |
| `linux-arm64` | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | Full AArch64 tests and archive smoke |
| `linux-musl-arm64` | `ubuntu-24.04-arm` | `aarch64-unknown-linux-musl` | Static archive build and smoke |
| `macos-x64` | `macos-15-intel` | `x86_64-apple-darwin` | Native workspace tests and archive smoke |
| `macos-arm64` | `macos-14` | `aarch64-apple-darwin` | Native workspace tests and archive smoke |
| `windows-x64` | `windows-latest` | `x86_64-pc-windows-msvc` | Native workspace tests and archive smoke |

Push a tag named:

```text
release-v<semver>
```

Tag-triggered releases intentionally use the `release-v<semver>` convention,
for example `release-v1.2.3` or `release-v1.2.3-rc.1`. Plain `v*` tags and
loose `release-*` tags are not release triggers. Commits pushed to `main` build
short-lived CI test archives but do not create GitHub release assets, and the
release workflow does not provide a manual dispatch path; a full release
requires a `release-v<semver>` tag on `main`.

For a tag build, the workflow creates a GitHub Release and uploads all archives
plus `SHA256SUMS.txt`, `slskr-cyclonedx.json`, and
`slskr-dependency-manifest.json`. The JSON manifests are included in the
release checksum file and build-provenance attestation subjects.
After the GitHub Release is published, the workflow posts a Discord
announcement using the `DISCORD_RELEASE_WEBHOOK_URL` repository secret.

The internal/unpublished Cargo crates intentionally remain at `0.0.0`. Binary
and archive version metadata comes from the release workflow. Tag builds derive
the artifact version from `release-v<semver>`, and local/manual archive tests
can still pass `SLSKR_RELEASE_VERSION` directly to
`scripts/build-release-archive.sh`.
Archive roots are named `slskr-<version>-<target>` so the published package
version remains tied to the release input even though the workspace crates are
not published independently.
