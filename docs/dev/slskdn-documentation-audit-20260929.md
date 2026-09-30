# slskdn Documentation Audit For The slskR Port

**Audit date:** 2026-09-29  
**slskR source:** c28d3f0370263341aceb0c0e8f096528088045a8  
**slskdn comparison:** 8f8e97aa (committed repository state)

This audit compares the root README and maintained documentation indexes, then
checks the relevant installation, feature, configuration, status, security,
deployment, and troubleshooting guides. slskdn is a useful source of topics,
but its instructions are not a specification for the Rust daemon. slskR has
its own runtime, configuration, network defaults, UI, release matrix, and API.

## Findings

### Already covered well in slskR

- The root README explains the product, core Soulseek workflows, release
  downloads, source builds, CLI commands, security defaults, and deployment
  shape.
- The maintained docs cover the install and service runbook, credential
  storage, the HTTP API and OpenAPI contract, four client libraries, webhooks,
  deployment and Kubernetes, release publishing, and focused guides for
  integrations and Gold Star Club.
- The Web UI and API have deeper route, lifecycle, interop, and test evidence
  than a user-facing feature list can provide. Those technical records should
  remain linked references, not be copied into a second giant README.

### Missing user-facing entry points

Before this audit, the documentation index had no single R-specific page for:

- A guided first run from a release archive through first login and first
  search.
- A user-oriented map of shipped features and the settings or external
  services they require.
- A clear status page separating ordinary operation, optional/configured
  behavior, compatibility-only behavior, and documented limits.
- A high-level architecture and network-port explanation.
- General troubleshooting for startup, login, network access, shares, UI
  assets, authentication, and transfers.
- A focused configuration guide connecting TOML, environment overrides,
  state paths, profiles, credentials, and API exposure.

The README gives fragments of most of this information, but users have to
assemble the operating path from a long overview and several technical pages.
The new guides provide direct paths and keep the README as the overview.

### R-specific correction required

Some existing port guidance no longer matched native/current behavior:

- The README described obfuscation bind variables as optional native listeners.
- The config example and app-surface page described DHT as independently using
  UDP 50300 even when the peer port was customized.
- The app-surface page described a distinct overlay bind as an available
  native/current choice.

Current native/current configuration uses the Soulseek peer port for ordinary
and obfuscated peer traffic and shares its numeric port across the DHT,
overlay, and QUIC UDP services. A mismatched dedicated bind is rejected.
Frozen compatibility profiles retain their historical behavior. The HTTP
Web UI/API listener remains a separate application listener. The README,
configuration example, app-surface notes, and new architecture guide are being
aligned with that behavior.

## Content deliberately not copied from slskdn

The following slskdn instructions describe its .NET application and must not
be presented as slskR behavior:

- ASP.NET hosting, YAML appsettings, default slskd login credentials, and its
  separate built-in HTTPS port.
- slskdn package-manager commands or distribution channels unless slskR's
  release and package files validate those exact claims.
- Feature maturity or roadmap claims inferred from slskdn screenshots, route
  names, service registrations, or design documents.
- Its VPN agent, Solid/OIDC, and other features without a corresponding
  implemented and documented slskR surface.

For slskR, the maintained configuration source is the annotated TOML example
plus supported environment overrides. The daemon starts with slskR-specific
commands and credential handling; its status and docs must describe the
behavior available in this repository.

## Work completed from this audit

- Add slskR-specific Getting Started, Features, Status, Configuration,
  Architecture, and Troubleshooting guides, with links from the README and
  documentation index.
- Correct the stale native/current peer, obfuscation, DHT, and overlay port
  descriptions.
- Retain the existing API, SDK, installation, deployment, security, and release
  guides as the authoritative detailed references for those areas.

This is an internal audit note. The added user and operator guides are the
release-facing documentation deliverable.
