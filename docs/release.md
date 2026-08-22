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

All compile-capable Cargo commands in this repository must use
`scripts/with-build-guard.sh`. The wrapper serializes Rust commands, forces one
Cargo job, and caps virtual memory at 12 GiB:

```sh
scripts/with-build-guard.sh cargo build --release -p slskr
scripts/with-build-guard.sh cargo test --workspace
```

The `slskr` package defaults to its bounded focused controller-test target.
The historical monolithic controller suite is rejected by the guard when
requested with `--features full-controller-tests` or `--all-features`, because
that profile previously exceeded the safe LLVM memory envelope. An explicit
`SLSKR_ALLOW_FULL_CONTROLLER_TESTS=1` opt-in is accepted only inside
`scripts/with-process-memory-guard.sh`, which adds the hard 4 GiB process cap;
direct or unguarded requests remain rejected. The default focused controller
test target remains the normal workspace test path.

Browser and frontend Node subprocesses in the parity and release gates use
`scripts/with-process-memory-guard.sh`. That guard applies a hard 4 GiB
resident-memory cgroup limit with swap disabled when a user systemd manager is
available and a 4 GiB virtual-memory limit otherwise. The release gate applies
that guard to each non-Rust step; its Cargo steps use the separate 12 GiB Rust
virtual-memory ceiling above, so a Rust formatter or compiler cannot inherit
the browser/Node process cap. This keeps browser, Node, and other heavy
non-Rust processes bounded without imposing their smaller limit on LLVM.

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

## CI Matrix

The release workflow builds these native runner variants:

- `linux-x64` on `ubuntu-latest`
- `linux-musl-x64` on `ubuntu-latest`
- `linux-arm64` on `ubuntu-24.04-arm`
- `linux-musl-arm64` on `ubuntu-24.04-arm`
- `macos-x64` on `macos-14` with the `x86_64-apple-darwin` Rust target
- `macos-arm64` on `macos-14`
- `windows-x64` on `windows-latest`

Push a tag named:

```text
release-v<semver>
```

Tag-triggered releases intentionally use the `release-v<semver>` convention,
for example `release-v1.2.3` or `release-v1.2.3-rc.1`. Plain `v*` tags and
loose `release-*` tags are not release triggers. Commits pushed to `main` do
not build release archives, and the release workflow does not provide a manual
dispatch path; a full release requires a `release-v<semver>` tag on `main`.

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
