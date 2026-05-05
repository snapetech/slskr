# Bug Burndown Ledger

Date: 2026-05-05

This ledger is the canonical council queue for security and bug audit work. `docs/security-bug-burndown.md` remains the narrative findings record; this file is the operational queue used to decide ownership, acceptance criteria, remediation requirements, and release-gate coverage.

## Council Operating Model

Reviewer roles are read-only. Reviewers may add evidence, challenge confidence, and recommend regression requirements, but implementation ownership stays with the relevant owner area.

| Role | Review scope |
| --- | --- |
| Backend/Security | Rust daemon/API, auth, storage, CSP, rate limits, webhook safety, and route behavior. |
| Frontend/API Handling | React web UI, Rust web UI, browser storage, navigation sinks, event streams, and API client behavior. |
| Release/Ops | CI, release workflows, provenance, package checks, dependency policy, and Kubernetes runtime posture. |
| Client SDKs | TypeScript, Python, and Go clients, URL construction, transport auth, tests, and packaging. |
| Network Runtime | Soulseek protocol/runtime, listeners, peer transport, live interop, VPN/public posture, and transfer/search pressure. |
| Adversarial Reviewer | Abuse cases, confused-deputy paths, SSRF, secret exposure, persistence surprises, and bypass attempts. |

## Status Rules

| Status | Meaning |
| --- | --- |
| Accepted | Council agrees the finding is real and remediation is required. |
| In Progress | An owner is actively implementing the remediation. |
| Fixed | Code/docs have changed and the regression requirement is satisfied locally. |
| Verified | Release-gate or CI coverage proves the fix remains in place. |
| Risk Accepted | Council explicitly accepts residual risk with a dated reason. |

## Domains

Allowed domains are `Backend/API`, `Rust Web UI`, `React Web UI`, `Client SDKs`, `Release/Ops`, `Kubernetes/Deployment`, `Network Runtime`, `Tests/Tooling`, and `Docs/Config`.

## Open Council Queue

| ID | Domain | Class | Severity | Confidence | Evidence | Impact | Owner Area | Regression Requirement | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BUG-001 | Backend/API | CSP | High | High | Static web responses require `'unsafe-inline'` and `wasm-unsafe-eval` in `crates/slskr/src/main.rs` and `crates/slskr/src/http_server.rs`. | XSS blast radius stays larger than needed for the bundled UIs. | Backend/Security + Rust Web UI | CSP tests that reject broad inline permissions for non-WASM shells and document any scoped WASM exception. | Accepted |
| BUG-002 | Release/Ops | Security scans | High | High | `scripts/run-release-gate.sh` has optional Semgrep/Trivy, while CI installs shellcheck/actionlint only. | Tag gates can skip security scans when tools are absent. | Release/Ops | CI either installs pinned Semgrep/Trivy or documents and enforces required/optional scan modes. | Accepted |
| BUG-003 | Client SDKs | WebSocket auth | High | High | `client-ts/src/websocket-client.ts` opens `/api/events/ws` with plain `new WebSocket(this.url)`. | Browser TypeScript clients cannot authenticate event streams in token-auth deployments. | Client SDKs + Backend/Security | Authenticated browser-safe event-stream ticket/subprotocol tests in SDK and server. | Accepted |
| BUG-004 | React Web UI | WebSocket auth | High | High | `web/src/lib/hubFactory.js` opens `/api/events/ws` with a plain browser WebSocket. | Main web event feed can silently fail when API token auth is enabled. | Frontend/API Handling + Backend/Security | Authenticated React event-feed E2E or unit coverage using the chosen browser-safe auth path. | Accepted |
| BUG-005 | Backend/API | Identifier predictability | Medium | Medium | Webhook IDs and event IDs are process-local counters in `crates/slskr/src/webhooks.rs`. | IDs can collide after restart if webhooks become persisted and are easy to guess. | Backend/Security | UUID/ULID or persistence-backed monotonic ID tests. | Accepted |
| BUG-006 | Backend/API | Webhook concurrency | Medium | High | Normal webhook dispatch spawns one Tokio task per matching webhook in `crates/slskr/src/webhooks.rs`; manual test sends are capped separately. | High event volume can accumulate outbound tasks and pressure runtime resources. | Backend/Security | Burst test proving dispatch uses a shared bounded queue/semaphore with observable drop/defer counts. | Accepted |
| BUG-007 | Backend/API | Outbound URL policy | Medium | High | Webhook validation blocks some private/loopback/link-local targets but lacks a centralized allow/deny CIDR policy and broader special-use coverage. | SSRF policy can drift as new outbound integrations are added. | Backend/Security + Adversarial Reviewer | Central outbound policy tests for IPv4/IPv6 loopback, private, link-local, documentation, multicast, and operator CIDRs. | Accepted |
| BUG-008 | Backend/API | Rate limiting | Medium | Medium | Rate limiting keys raw peer socket address and token digest in `crates/slskr/src/main.rs`. | Reverse-proxy deployments may over-throttle many clients as one IP. | Backend/Security | Trusted proxy parsing tests with explicit allowlist and spoofing rejection cases. | Accepted |
| BUG-009 | Backend/API | Storage pressure | Medium | High | Recursive downloads/incomplete listing can walk up to 16,384 entries per request. | Repeated recursive scans can create avoidable CPU/disk pressure. | Backend/Security + Network Runtime | Pagination/lower-budget tests and rate-limit coverage for recursive storage scans. | Accepted |
| BUG-010 | Docs/Config | Compatibility persistence | Medium | High | `/api/options` mutation compatibility routes are validated and non-persisted, but clients may assume durable mutation. | Operators and clients can misread compatibility acknowledgements as saved config. | Docs/Config + Backend/Security | OpenAPI/docs explicitly mark non-persistent acknowledgements, with route tests for read-only behavior. | Accepted |
| BUG-011 | Docs/Config | Compatibility no-op inventory | Medium | High | Preserved parity routes include logs/cache/bridge/config/bans/share-grant token and MusicBrainz subscription capability shells. | Compatibility endpoints may look fully supported when they are intentional no-ops or empty shells. | Docs/Config + Backend/Security | Inventory each compatibility acknowledgement/empty shell in docs/OpenAPI and assert the advertised shape in tests. | Accepted |
| BUG-012 | Release/Ops | Release SBOM | Medium | High | Release assets are attested but no SBOM or dependency/license manifest is attached. | Consumers cannot audit release dependencies from published assets alone. | Release/Ops | Release workflow publishes CycloneDX/SPDX or equivalent manifests with attestations. | Accepted |
| BUG-013 | Release/Ops | Cargo package verification | Medium | Medium | Full `cargo package --workspace` verification fails on current Cargo temporary registry handling for internal workspace crates. | Release packaging remains less hermetic than intended. | Release/Ops | Restore full package verification or add unpack/build verification that catches missing package inputs. | Accepted |
| BUG-014 | Release/Ops | Release tag policy | Medium | High | Release workflow triggers on `release-*`, while semantic `v*` tags are not accepted. | Tag conventions can diverge from operator expectations and docs. | Release/Ops | Release docs and workflow enforce one accepted tag convention. | Accepted |
| BUG-015 | Release/Ops | Version metadata | Medium | High | Workspace crates are `0.0.0` while artifacts derive a separate version label. | Package metadata and binary release names can disagree. | Release/Ops | Release check proves artifact names, binary version, and crate metadata are intentionally aligned or documented. | Accepted |
| BUG-016 | Release/Ops | Secret scanning | Medium | High | Ignored local secret files exist, but CI/release has no pinned secret scan. | Accidental secret commits depend on review discipline. | Release/Ops + Adversarial Reviewer | Pinned secret scanner in CI/release with placeholder allowlist coverage. | Accepted |
| BUG-017 | Docs/Config | OpenAPI drift | Medium | High | API parity work changes response shapes faster than `docs/openapi.json` and docs. | Generated clients and operators can depend on stale contracts. | Docs/Config + Tests/Tooling | Generated OpenAPI/docs drift check fails when checked-in docs differ. | Accepted |
| BUG-018 | Tests/Tooling | Compatibility smoke coverage | Medium | Medium | slskd API compatibility smoke is opt-in because it needs live-style setup and external package install. | Compatibility regressions can miss normal CI. | Tests/Tooling | Scheduled CI or hermetic fixtures run compatibility smoke without blocking local contributors. | Accepted |
| BUG-019 | Client SDKs | Python client gate | Medium | High | Python client lacks lint/type/test/audit gate. | Python SDK regressions can ship unnoticed. | Client SDKs | Add ruff, pyright or mypy, pytest smoke tests, and dependency audit. | Accepted |
| BUG-020 | Client SDKs | Go/Python CI coverage | Medium | High | CI/release gate install pinned Go/Python and run `scripts/check-client-sdk-gates.sh`, which runs `go test ./...` plus Python compile/import coverage. | SDK compile/runtime regressions fail CI and release gates. | Client SDKs + Release/Ops | Pinned Go/Python setup plus `go test ./...` and Python checks in CI/release. | Verified |
| BUG-021 | Release/Ops | Rust dependency hygiene | Medium | Medium | `cargo tree -d` shows duplicate dependency families in the release graph. | Binary size and dependency review surface stay larger than needed. | Release/Ops | Scheduled dependency report and consolidation plan where semver permits. | Accepted |
| BUG-022 | Tests/Tooling | Audit tooling availability | Medium | Medium | `cargo outdated` and `cargo udeps` were absent locally and not provisioned by the gate. | Dependency freshness and unused-dependency checks are not reproducible. | Tests/Tooling | Pinned tooling or CI-native replacements such as cargo-deny/cargo-machete/scheduled outdated reports. | Accepted |
| BUG-023 | Release/Ops | GitHub Actions supply chain | Medium | High | CI/release use mutable action tags such as `actions/checkout@v4`, `actions/setup-node@v4`, and `softprops/action-gh-release@v2`. | Workflow dependency trust changes without an explicit review point. | Release/Ops | Actions pinned to reviewed SHAs with automated update policy. | Accepted |
| BUG-024 | Network Runtime | Transfer event growth | Low | Medium | Transfer event history appends indefinitely and is recreated only when absent. | Long-running nodes can accumulate unbounded transfer event logs. | Network Runtime | Rotation or compaction tests tied to transfer history limit or byte cap. | Accepted |
| BUG-025 | Backend/API | Rust module hygiene | Low | High | Broad `#![allow(dead_code)]` appears in Rust modules. | Unused compatibility code can accumulate without review pressure. | Backend/Security | Remove broad allowances or gate intentional compatibility helpers with tests/features. | Accepted |
| BUG-026 | Tests/Tooling | Script dependencies | Low | High | Release/live scripts assume many tools without a preflight summary. | Operators hit late failures without actionable missing-tool context. | Tests/Tooling | `scripts/check-dev-tooling.sh` or script-local preflights for live/release paths. | Accepted |
| BUG-027 | Tests/Tooling | Test noise | Low | Medium | Web tests emit repeated jsdom navigation warnings. | Real test failures are harder to spot. | Frontend/API Handling | Central test setup mocks navigation/reload paths. | Accepted |
| BUG-028 | Release/Ops | Deprecated npm transitives | Low | Medium | npm install warns on deprecated packages including old core-js and Babel proposal packages. | Security and maintenance drift persists in frontend dependency graph. | Release/Ops + Frontend/API Handling | Dependency owner upgrades or documented exceptions with advisory checks. | Accepted |
| BUG-029 | Release/Ops | Dependency modernization | Low | Medium | npm outdated shows major-version drift across web/dashboard/client-ts. | Future security fixes may become harder to apply. | Release/Ops + Frontend/API Handling | Planned compatibility upgrade track with UI/client regressions. | Accepted |
| BUG-030 | Docs/Config | Stale docs | Low | High | Docs outside deployment guide still contain stale localhost, legacy config, image tag, and wildcard CORS examples. | Operators may copy insecure or obsolete deployment examples. | Docs/Config | Grep-based docs freshness check plus doc updates or archiving. | Accepted |

## Council Gate Mapping

| Gate | Purpose |
| --- | --- |
| `scripts/check-endpoint-parity-drift.sh` | Preserve current parity endpoint coverage and fail if the canonical endpoint list drifts from the router. |
| `scripts/check-browser-token-persistence.sh` | Prevent browser API tokens and ListenBrainz tokens from returning to persistent `localStorage`. |
| `scripts/check-unsafe-blank-opens.sh` | Prevent reverse-tabnabbing regressions in browser links and `window.open` calls. |
| `scripts/check-websocket-auth-coverage.sh` | Keep browser WebSocket auth gaps visible until the accepted client/UI findings are fixed. |
| `scripts/check-webhook-outbound-policy.sh` | Keep webhook registration validation and the accepted outbound-policy/concurrency findings visible. |
| `scripts/check-workflow-release-policy.sh` | Enforce current release workflow hardening and keep accepted release-policy findings registered. |
| `scripts/check-package-artifact-matrix.sh` | Verify release archive targets and keep SBOM/version/package gaps registered. |
| `scripts/check-client-sdk-gates.sh` | Run Go SDK tests and Python SDK compile/import checks in CI and release gates. |
| `scripts/check-openapi-docs-drift.sh` | Validate checked-in OpenAPI JSON and keep OpenAPI/compatibility documentation gaps registered. |
| `scripts/check-shell-script-hygiene.sh` | Run shell syntax checks and flag common script footguns. |
| `scripts/check-kubernetes-public-posture.sh` | Enforce single-replica, PVC-backed, authenticated-metrics, restricted Kubernetes defaults. |
| `scripts/check-compatibility-noop-documentation.sh` | Keep preserved compatibility acknowledgements documented and tracked until route docs/tests fully cover them. |
| `scripts/check-remediation-script-registry.sh` | Ensure every `scripts/check-*.sh` gate is executable and registered in the baseline. |
