# slskr Documentation

This index is the maintained map for repository docs. The root
[README](../README.md) is the project overview; this file is the table of
contents for operators, API users, UI contributors, release work, and parity
audits.

## Start Here

| Document | Use it for |
| --- | --- |
| [Project README](../README.md) | Product overview, screenshots, quick start, feature summary, and top-level links. |
| [Install guide](install.md) | Build, install, service, container, config, state, and exposure runbook. |
| [Credential storage](credential-storage.md) | Soulseek credential-source choices, systemd credentials, Web UI behavior, and security notes. |
| [Config example](slskr.config.example.toml) | Annotated TOML config with the supported runtime settings. |
| [App surface](app-surface.md) | User-facing CLI, daemon, API, Web UI, auth, and compatibility surface. |

## Web UI

| Document | Use it for |
| --- | --- |
| [Web UI README](../web/README.md) | React/Vite Web UI development, audit, build, and screenshot workflow. |
| [README screenshots](screenshots/) | Generated screenshots used by the root README. |
| [Web UI endpoint inventory](webui-endpoints.txt) | Route inventory used during parity and audit work. |
| [Rust Web UI notes](rust-web-ui.md) | Rust/WASM migration target notes; not the currently shipped browser bundle. |
| [Rust UI parity ledger](rust-ui-parity-ledger.md) | UI parity and migration audit ledger. |

The currently shipped browser UI is the React/Vite app in `web/`. Release
archives package the built assets and `slskr serve` serves them on the same
listener as the HTTP API.

## HTTP API And Automation

| Document | Use it for |
| --- | --- |
| [HTTP API reference](http-api.md) | Maintained route reference and request/response examples. |
| [HTTP API features](http-api-features.md) | Capability notes, event stream details, and workflow examples. |
| [HTTP API deployment](http-api-deployment.md) | Auth, reverse proxy, exposure, and Kubernetes deployment notes. |
| [OpenAPI document](openapi.json) | Machine-readable API contract. |
| [Webhook API](WEBHOOK_API.md) | Webhook subscription, delivery, signing, and retry behavior. |
| [Client libraries](CLIENT_LIBRARIES.md) | Cross-language SDK overview. |
| [TypeScript client](../client-ts/README.md) | `@slskr/api-client` usage. |
| [Python client](../client-python/README.md) | Python client usage. |
| [Go client](../client-go/README.md) | Go client usage. |
| [Examples](../examples/README.md) | End-to-end API and automation examples. |
| [GraphQL schema](GRAPHQL_SCHEMA.graphql) | GraphQL contract notes for integration surfaces. |

Unless a document says otherwise, local examples assume the default daemon URL:

```text
http://127.0.0.1:5030
```

## Operations, Release, And Security

| Document | Use it for |
| --- | --- |
| [Release guide](release.md) | Tag, archive, manifest, checksum, and verification policy. |
| [Live interop matrix](live-interop-test-matrix.md) | Live Soulseek-network and cross-client verification gates. |
| [VPN certification](vpn-certification.md) | Per-account VPN isolation, credential pool setup, and certification runner. |
| [Full network test plan](full-network-test-plan.md) | Test phases, pass criteria, and release-readiness certification plan. |
| [Security bug burndown](security-bug-burndown.md) | Security audit findings and remediation status. |
| [Compliance](../COMPLIANCE.md) | Project compliance and public posture notes. |
| [Remediation plan](../REMEDIATION.md) | Historical remediation tracking. |
| [Project plan](../PLAN.md) | Planning notes and backlog context. |
| [GitHub Actions pin policy](dev/github-actions-pin-policy.md) | CI action pinning policy. |
| [Rust dependency hygiene](dev/rust-dependency-hygiene.md) | Rust dependency review guidance. |

## Fixtures, Parity, And Performance

| Document | Use it for |
| --- | --- |
| [Open commons fixtures](open-commons-fixtures.md) | Fixture source policy and manifest notes. |
| [Legacy port harvest](legacy-port-harvest.md) | Historical implementation and parity harvest notes. |
| [Runtime parity ledger](parity/slskdn-slsknet-runtime-parity.md) | slskdN/Soulseek.NET-family behavior parity ledger. |
| [Performance analysis](performance-analysis.md) | Performance observations and follow-up areas. |
| [Enhancements](ENHANCEMENTS.md) | Historical enhancement backlog. |
| [Integration guide](INTEGRATION_GUIDE.md) | Historical component-integration backlog; use current API and Web UI docs for runbooks. |

## Developer Audit Notes

The `docs/dev/` directory holds audit ledgers and internal review playbooks:

| Document | Use it for |
| --- | --- |
| [Bug council playbook](dev/council-bughunt-playbook.md) | Review and bug-hunt process. |
| [Council scan inventory](dev/council-scan-inventory.md) | Cross-cutting scan inventory. |
| [Bug burndown ledger](dev/bug-burndown-ledger.md) | Bug remediation ledger. |
| [Active backlog](dev/bug-council-active-backlog.md) | Open audit backlog. |
| [Severity schema](dev/bug-council-severity-schema.md) | Finding severity definitions. |
| [Scan registry](dev/bug-council-scan-registry.md) | Scan registry and ownership notes. |
| [Behavior pinning](dev/bug-council-behavior-pinning.md) | Behavior pinning guidance. |
| [Negative space](dev/bug-council-negative-space.md) | Missing-test and omitted-surface review notes. |
| [Sibling search](dev/bug-council-sibling-search.md) | Similar-code search process. |
| [Adversarial fuzz](dev/bug-council-adversarial-fuzz.md) | Fuzz and adversarial-input review notes. |
| [Roslyn analyzers](dev/bug-council-roslyn-analyzers.md) | Legacy analyzer notes retained for migration context. |
| [Bug council phases](dev/bug-council-phases.md) | Audit phase plan. |

## Documentation Status

- The canonical operator entry points are the root README, this index,
  [install.md](install.md), [app-surface.md](app-surface.md), and
  [http-api.md](http-api.md).
- The shipped Web UI is documented in [../web/README.md](../web/README.md).
  Rust/WASM UI docs are retained as migration notes until that target replaces
  the React bundle.
- Historical planning documents are retained because they explain why surfaces
  exist, but they are not deployment runbooks unless this index marks them as
  maintained operator docs.
