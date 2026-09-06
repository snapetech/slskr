# Changelog

All notable changes to slskr are documented here. Release pages are generated
from the tagged source by the release workflow, with structured fragments from
`release-notes/` assembled into the user-facing release section.

Use release sections in this form:

```markdown
## [<version>] — YYYY-MM-DD
```

Keep the file append-only at the release-section level. Add shipped user-facing
bullets to `## [Unreleased]`, then move them into the dated version section when
the release is prepared. Do not rewrite audited release history.

---

## [Unreleased]

## [0.2.39] — 2026-09-04

- Expanded required CI to build and verify every supported Linux, macOS, and
  Windows archive on current changes, with native workspace tests where
  available and downloadable test artifacts.
- Repaired Winget fork synchronization with recoverable backups, and aligned
  package metadata, architecture-aware installers, Nix sources, and release
  documentation with the actual artifact matrix.
- Expanded downstream packaging and deployment validation across Linux
  amd64/AArch64, native macOS, Snap, Flatpak, RPM, Debian, containers, and
  supported operator manifests.

## [0.2.38] — 2026-08-30

- Published native Linux x86-64 and AArch64 GNU/musl, macOS Intel and Apple
  silicon, and Windows archives with checksums, dependency manifests, and
  build provenance.
- Hardened compatibility-profile routing, Soulseek transport, transfer and
  persistence lifecycles, and the React Web UI's error and reconnect paths.
- Refreshed downstream package metadata and release validation for the current
  archive matrix.

## [0.2.34] — 2026-08-28

- Aligned native/current networking with current upstream shared-port behavior,
  including bounded Soulseek/TLS mesh TCP demultiplexing and VPN-forwarded
  port updates.
- Improved Web UI identity and interactions, including login, Dropdown and
  Modal behavior, sharing access, stream permissions, and grant contracts.
- Hardened Rust and process memory guards, formatting/build tooling, and
  release and browser validation workflows.
- Fixed release validation to resolve frozen slskd authentication checks
  against the renamed legacy policy registry.
- Fixed options differential validation to ignore only known default product
  identity fields while preserving strict checks for configured values.
- Preserved the historical dedicated obfuscated listener for frozen native
  compatibility profiles while keeping current native deployments on the
  merged public listener.
- Preserved frozen-profile Lidarr import request compatibility while retaining
  the current upstream already-owned album pre-check in current native mode.

## [0.2.33] — 2026-08-23

- Fixed share scanning for large libraries with thousands of top-level
  directories while retaining bounded resource use and symlink safety.
- Matched frozen-profile configuration-watch behavior for malformed YAML by
  clearing the transient options projection while retaining the live share
  index until a valid reload.
- Improved VPN-scoped live validation for listener metadata, browse, API
  authentication, queued transfers, hash verification, and session soak.
- Expanded public Soulseek acceptance across the available credential pool,
  including plain, obfuscated, distributed, indirect, messaging, and transfer
  paths.

## [0.2.32] — 2026-08-21

- Added bounded release, parity, interop, and browser-audit workflows with
  structured release-note assembly and multi-platform archive publication.
- Fixed compatibility-profile routing, protocol/session stability, persistence
  contention handling, and the React Web UI's error, CSP, modal, and automation
  surfaces.
- Added live MusicBrainz, discovery, MediaCore, Soulseek, transfer, messaging,
  and system-surface coverage to the acceptance evidence.
- Published release pages now use a concise proof-linked summary while retaining
  the complete structured fragment assembly as a downloadable release asset.
