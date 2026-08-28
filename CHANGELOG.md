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
