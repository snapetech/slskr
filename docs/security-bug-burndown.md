# Security And Bug Burn-Down

Date: 2026-05-05

Scope: current `slskR` checkout, including Rust daemon/API, Rust WASM UI, React web UI, standalone dashboard, TypeScript/Python/Go clients, scripts, CI, release, and Kubernetes manifests.

## Fixed In This Pass

| Severity | Area | Finding | Status |
| --- | --- | --- | --- |
| High | Backend integrations | Lidarr SSRF validation resolved hosts but did not pin the resolved address for the outbound request, leaving DNS rebinding risk between validation and request. | Fixed by pinning resolved Lidarr addresses into the reqwest client. |
| High | CI/release | CI only ran Rust checks, so web, dashboard, TypeScript client, wasm, and advisory regressions could merge. | Fixed by expanding CI to the full release-gate surface. |
| High | Release provenance | Release publishing produced checksums but no GitHub artifact attestations. | Fixed with build provenance attestations and required permissions. |
| Medium | Dashboard auth | Standalone dashboard persisted API keys in `localStorage`. | Fixed by keeping API keys in `sessionStorage`; API URL remains persistent. |
| Medium | Client logging | TypeScript client debug logging printed request bodies, which can include secrets. | Fixed with recursive secret-field redaction. |
| Medium | Test script auth | slskd API compatibility smoke had a deterministic default API token. | Fixed by requiring `SLSKR_SLSKD_API_SMOKE_TOKEN`. |
| Medium | API auth bootstrap | `/api/session/enabled` could require auth even though login bootstrap calls it before a token exists. | Fixed by making it explicitly public and adding route coverage. |
| Medium | Frontend session handling | Web session check masked network failures with a secondary `error.response.status` TypeError. | Fixed with optional response handling and a regression test. |
| Medium | Kubernetes RBAC | The app ServiceAccount had pod/configmap read/list/watch permissions that the deployment does not need. | Fixed by removing the unused Role and RoleBinding. |
| Medium | Monitoring | ServiceMonitor labels did not match the Prometheus selector in `prometheus-values.yaml`. | Fixed by adding `prometheus: "true"`. |
| Low | Docs | Security docs said the dashboard saves API tokens in a cookie, which no longer describes the preferred browser behavior. | Fixed to document bearer/session-storage behavior and legacy cookie compatibility. |
| Low | Release setup | Release gate setup cached only the web lockfile and did not install the wasm target in the gate job. | Fixed by adding wasm and all npm lockfiles. |
| Low | API parity | slskd transfer report routes referenced missing helpers in this checkout. | Fixed by preserving the routes and adding report helper implementations. |
| High | Kubernetes runtime | `slskr-api` ran three replicas against a daemon that owns a live Soulseek session and local in-memory/session state. | Fixed default manifests to run one API replica and constrained the API HPA to one replica. |
| High | Kubernetes storage | `/data` used `emptyDir`, losing API state on pod restart and making replicated state inconsistent. | Fixed default manifests to mount a `slskr-data` PVC. |
| High | Browser token persistence | Main React web UI supported `rememberMe` bearer-token persistence in `localStorage`. | Fixed by removing the login persistence toggle, storing login tokens in `sessionStorage`, and ignoring legacy persistent tokens. |
| High | Legacy cookie auth | Backend accepted `slskr.session` cookies whenever API-token auth was enabled. | Fixed by adding `SLSKR_API_COOKIE_AUTH_ENABLED` / `[auth].cookie_auth_enabled`, defaulting legacy cookie auth off while preserving an explicit compatibility opt-in. |
| High | External process launch | `/api/player/external-visualizer/launch` could spawn the configured local command whenever a command was configured. | Fixed by requiring separate `SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED=true` opt-in and recording launch/blocked/failure events. |
| Medium | Webhook registration | Webhook URL validation happened only at delivery/test time, so obviously blocked URLs could be saved. | Fixed by validating registration and admin creation URLs for scheme, host presence, localhost, direct private/link-local IP targets, and known ports. |
| Medium | Webhook secret lifecycle | Webhook creation returned the secret without documenting that it is a one-time creation-only value. | Fixed by adding `docs/WEBHOOK_API.md`, marking create responses with `secretReturnedOnce`, and documenting that list/detail/log routes omit secrets. |
| Medium | Auth disabled escape hatch | `SLSKR_AUTH_DISABLED=true` could expose protected APIs on non-loopback binds without a health-surface warning. | Fixed by adding a startup warning and `/api/health` warning when auth is disabled on a non-loopback bind. |
| Medium | Path deletion parity | Download/incomplete file deletion compatibility routes returned success stubs instead of scoped deletion. | Fixed by decoding slskd path segments, rejecting traversal/absolute paths/symlinks, and deleting only under the downloads or incomplete storage root. |
| Medium | Path listing parity | Download/incomplete directory compatibility routes returned empty shells and recursive listings had no traversal budget. | Fixed by listing scoped storage roots, rejecting traversal/symlink escapes, supporting recursive responses, and capping listings at 16,384 entries. |
| Medium | Static asset serving | Configured web build assets followed symlinks and were read into memory without a size cap. | Fixed by canonicalizing resolved assets under the build root, rejecting symlink escapes, and capping static asset reads at 16 MiB. |
| Medium | Config mutability | `/api/options` mutation compatibility routes accepted arbitrary bodies while reporting success. | Fixed by returning sanitized runtime config from `GET /api/options` and requiring valid JSON-object bodies for read-only mutation acknowledgements. |
| Medium | Config compatibility endpoints | `/api/options/yaml/location` returned a fake `/etc/slskr/config.yaml` path and YAML upload/validate routes accepted non-string JSON bodies. | Fixed by reporting the actual loaded config path or `runtime://memory`, returning read-only TOML compatibility text, and rejecting non-string upload/validate payloads. |
| Low | Config loading | Startup config loading read the entire TOML file before parsing. | Fixed by rejecting config files larger than 1 MiB before reading into memory. |
| Low | Archive verification | `verify-release-artifacts.sh` extracted zip files without path traversal checks. | Fixed by rejecting absolute and parent-directory zip entries before extraction. |
| Low | Kubernetes secrets | Manifest references `slskr-secrets` and `grafana-admin` without templates. | Fixed by adding `k8s/secrets.example.yaml` with placeholder-only Secret manifests. |
| Medium | CI tooling | Local gate lacked workflow/shell/security tool hooks beyond Rust/npm advisory checks. | Fixed by adding optional local `shellcheck`, `actionlint`, `semgrep`, and `trivy` release-gate steps plus CI setup for shellcheck/actionlint. |
| Medium | Docs | `docs/http-api-deployment.md` contained stale config names, `rust:latest`, `slskr:latest`, and wildcard CORS examples. | Fixed by replacing it with current `SLSKR_*` config, reverse-proxy, Kubernetes, metrics, and no-wildcard-CORS guidance. |
| Medium | Metrics docs | Metrics path guidance was split between `/api/metrics` and `/api/v0/metrics`. | Fixed deployment docs to state both aliases and that Kubernetes scrape config uses `/api/metrics`. |
| High | Kubernetes metrics | ServiceMonitor scraped protected `/api/metrics` without authentication and referenced an unused metrics service port. | Fixed by scraping the real `http` port with `SLSKR_API_TOKEN` from `slskr-secrets`, removing unauthenticated scrape annotations, and dropping the unused metrics port/config. |
| High | Go client | `client-go/client.go` defined `SendMessage` twice, preventing the Go client from compiling. | Fixed by removing the duplicate method; Go toolchain is still needed in CI to enforce `go test ./...`. |
| Low | Built-in dashboard token field | The embedded fallback dashboard rendered the API token input as a text field. | Fixed by using a password input with token-safe browser attributes. |
| Low | Language accounting | `.gitattributes` marked `web/`, `dashboard/`, and `client-ts/` as vendored, hiding maintained JavaScript/TypeScript source from GitHub language stats. | Fixed by counting maintained source and excluding only generated build, coverage, dependency, and lockfile artifacts. |
| Low | Config file type | Config loading capped config file size but did not reject non-regular paths before reading. | Fixed by requiring regular files and adding directory rejection coverage. |
| Low | HTTP 413 reason phrase | Oversized request bodies were rejected with non-standard `413 Content Too Large`. | Fixed by returning `413 Payload Too Large`, matching static asset oversize responses. |
| Medium | API pagination | Earlier audit notes listed unbounded list limits, but `RecordListFilter` now defaults and clamps requested limits to `DEFAULT_LIST_LIMIT` (`crates/slskr/src/main.rs:845`, `crates/slskr/src/main.rs:864`) with regression coverage for omitted, huge, and zero limits. | Verified as fixed; keep route-family regression tests in place when adding new list endpoints. |
| Medium | Webhook secrets | Webhook creation accepted caller-supplied signing secrets without minimum strength checks. | Fixed by requiring supplied secrets to be at least 32 bytes, printable, and have basic character variety on public/admin creation routes while preserving generated secrets by default. |
| Medium | Frontend auth passthrough | `session.authHeaders()` emitted `Authorization: Bearer n/a` in passthrough mode. | Fixed by omitting Authorization for passthrough tokens and adding regression coverage. |
| Medium | Vite Less alias traversal | The Less alias file manager resolved `~` imports without checking that the result stayed under `node_modules`. | Fixed by rejecting absolute and escaping alias imports before reading. |
| Medium | Web build script failure masking | `SLSKR_BUILD_WEB` logged npm build failures but allowed cargo builds to continue. | Fixed by failing the build script on requested web build failures and including stdout/stderr. |
| Low | Service worker cache scope | Service worker activation deleted every cache key on the origin. | Fixed by deleting only cache names with the `slskr-` prefix and adding activation coverage. |
| Medium | TypeScript client path escaping | The TypeScript client interpolated IDs, usernames, and room names directly into URL paths. | Fixed by encoding all client-composed dynamic path segments with `encodeURIComponent`. |
| Medium | Python client path escaping | The Python client interpolated path segments and used `urljoin`, allowing caller-controlled path rewriting. | Fixed by encoding dynamic path segments with `quote(..., safe="")` and joining against the configured base URL directly. |
| Medium | Python WebSocket lifecycle | Python WebSocket connect created an anonymous `aiohttp.ClientSession` and retained only the WebSocket response. | Fixed by storing and closing the session on disconnect and failed connect attempts. |
| Medium | Go WebSocket read limits | Go WebSocket event handling used `ReadMessage()` without a read limit. | Fixed by setting a 64 KiB read limit after dialing, matching the server event frame cap. |
| Low | Go client URL escaping | Go client methods interpolated usernames, message IDs, and room IDs directly into paths. | Fixed by escaping path parameters with `url.PathEscape`. |
| Low | Client error redaction | Go client errors included raw response bodies that could echo upstream secrets. | Fixed by redacting common secret fields from JSON and text error bodies before returning errors. |
| Medium | Frontend prototype pollution | Adversarial settings used dynamic nested object writes and only guarded two of the array/object update helpers. | Fixed by rejecting `__proto__`, `constructor`, and `prototype` paths in all nested setting update helpers. |
| Low | State write atomicity | Share cache and transfer state used direct truncating writes. | Fixed by writing state snapshots to a synced temp file in the same directory and renaming into place. |
| Low | Config secret permissions | TOML config files can contain credentials and API tokens without warning when group/world-readable. | Fixed by warning on Unix when a config file with known secret fields has group/other permission bits set. |
| Medium | Webhook delivery DoS | `/api/webhooks/:id/test` could spawn unbounded delivery tasks. | Fixed by sharing a bounded webhook delivery semaphore and returning `429` when the delivery pool is full. |
| Medium | Webhook identifiers | Webhook IDs and event IDs were process-local counters. | Fixed by generating UUIDv4-backed `hook_` and `evt_` identifiers with regression coverage and remediation-gate checks. |
| Medium | Webhook dispatch concurrency | Normal webhook event dispatch spawned per-webhook delivery tasks without acquiring the shared delivery pool. | Fixed by passing the shared webhook delivery semaphore into normal dispatch and dropping over-capacity deliveries before outbound work starts. |
| Medium | Webhook SSRF policy | Webhook validation lacked a central allow/deny CIDR policy and complete special-use coverage. | Fixed with a shared outbound policy, IPv4/IPv6 documentation and multicast blocks, operator allow/deny CIDR hooks, docs, tests, and remediation-gate coverage. |
| Medium | Rate limiting | Anonymous rate limiting keyed every reverse-proxied client by the proxy socket address. | Fixed by adding trusted proxy CIDR configuration, parsing `Forwarded` and `X-Forwarded-For` only from allowlisted proxies, and adding spoofing rejection tests/gate coverage. |
| Medium | Storage listing pressure | Recursive downloads/incomplete directory listing could walk up to 16,384 entries per request. | Fixed by adding `limit`/`offset` storage listing options, stable traversal order, lower recursive defaults, a 1,024-entry recursive cap, truncation metadata, and remediation-gate coverage. |
| High | Release workflow | The release version step interpolated GitHub context expressions directly inside a shell script. | Fixed by passing workflow context through `env:` and quoting shell variables. |
| High | Kubernetes release artifacts | Default manifests deployed a standalone dashboard image that release CI does not publish. | Fixed by removing standalone dashboard resources and routing `/` to the API service that serves the embedded UI. |
| Medium | Release package check | Release package verification used `--allow-dirty --no-verify`, masking dirty release state. | Fixed by removing `--allow-dirty`; full Cargo tarball verification still hits a Cargo internal workspace-registry limitation and remains tracked below. |
| Low | Kubernetes hardening | API pods omitted `runAsGroup`, `seccompProfile`, and disabled service-account-token automounting. | Fixed by setting restricted pod/container security context fields and `automountServiceAccountToken: false`. |
| Low | Kubernetes availability | The single-replica API PDB allowed one unavailable pod. | Fixed by requiring `minAvailable: 1`. |
| Low | Tar archive verification | Release artifact verification extracted tar archives without member safety checks. | Fixed by validating tar paths, rejecting links/special files, and extracting only after all members pass. |
| Medium | Frontend reverse tabnabbing | Several `_blank` opens omitted `noopener`/`noreferrer`. | Fixed with a shared safe-open helper that uses `noopener,noreferrer`, clears `opener`, and covers stream/privilege links. |
| Medium | Release reproducibility | CI installed `actionlint` with `go install ...@latest`. | Fixed by pinning the actionlint version used by CI and release gates. |
| Medium | Release concurrency | Release workflow had no concurrency group. | Fixed with a ref-scoped release concurrency group and cancellation disabled. |
| Medium | Browser token persistence | ListenBrainz user tokens were saved in persistent `localStorage`. | Fixed by storing ListenBrainz tokens in `sessionStorage` and updating UI regression coverage. |
| Medium | Endpoint parity tooling drift | Endpoint parity tooling reported implemented conversation routes as missing and included malformed `GET /conversations:var`. | Fixed by removing the malformed manifest entry and teaching the checker about `path_segment_after` dynamic handlers. |
| Low | Kubernetes NetworkPolicy | Manifests did not define ingress/egress NetworkPolicies. | Fixed by adding an API pod NetworkPolicy with explicit ingress and scoped DNS/web/Soulseek egress. |
| Low | Kubernetes image freshness | Default manifests used `IfNotPresent` with release-placeholder tags. | Fixed by switching the API image pull policy to `Always` while placeholder tags remain mutable. |
| Medium | External metadata privacy | Lyrics lookup could persist open state and automatically send current artist/title metadata to LRCLIB later. | Fixed by making lyrics lookup a per-session explicit action and ignoring stale persisted lyrics-open state. |
| Medium | Go/Python client CI coverage | CI and the release gate did not install Go or run Go/Python client checks. | Fixed by adding pinned Go/Python setup and `scripts/check-client-sdk-gates.sh`, which runs `go test ./...` plus Python compile/import coverage. |
| Low | Script dependencies | Release/live scripts assumed many local tools without a preflight summary. | Fixed by adding `scripts/check-dev-tooling.sh` to the remediation baseline so required tools fail fast and optional live/security tools are summarized. |
| Low | Test noise | Web tests emitted repeated jsdom navigation warnings from the unauthorized reload path. | Fixed by skipping page reload only under Vitest `MODE=test` and adding regression coverage for test and production behavior. |
| High | Browser WebSocket auth | Browser clients opened `/api/events/ws` without a bearer-capable auth mechanism. | Fixed by accepting a `slskr.api-token.<percent-encoded-token>` WebSocket subprotocol on the server and using it from the TypeScript SDK. |
| High | Main web event feed auth | The React web event hub opened `/api/events/ws` without a browser-safe token path. | Fixed by sending the same auth subprotocol for session bearer tokens, omitting passthrough/missing tokens, and adding Vitest coverage. |
| High | Release security scans | Semgrep and Trivy were optional local checks and absent from CI/tag-gate required mode. | Fixed by adding `scripts/run-security-scans.sh`, required CI/release mode, pinned scanner images, and workflow policy coverage while keeping local scanner infrastructure non-blocking by default. |
| High | CSP | Static and generic responses used broad inline script/style allowances, and the Rust WASM shell used an inline module bootstrap. | Fixed by adding strict or nonce-backed generic CSP, moving the Rust WASM bootstrap to a static module, rejecting broad inline allowances for static web assets, and scoping `wasm-unsafe-eval` only to Rust WASM builds. |

## Open Burn-Down

| Severity | Area | Finding | Proposed fix |
| --- | --- | --- | --- |
| Medium | Config persistence | Options/config compatibility mutation routes are validated and clearly non-persisted (`crates/slskr/src/main.rs:4591`), but clients may assume durable mutation. | Implement a schema-validated config writer or document/OpenAPI-mark these routes as non-persistent compatibility acknowledgements. |
| Medium | Compatibility no-op inventory | Several preserved slskd parity routes return compatibility acknowledgements or empty capability shells, including logs/cache/bridge/config/bans/share-grant token and MusicBrainz subscription routes (`crates/slskr/src/main.rs:4458`, `crates/slskr/src/main.rs:5721`, `crates/slskr/src/main.rs:7585`, `crates/slskr/src/main.rs:8216`, `crates/slskr/src/main.rs:8616`, `crates/slskr/src/main.rs:8663`). | Preserve the endpoints, but mark exact persistence/support behavior in OpenAPI/docs and add tests that assert the advertised non-persistent shape. |
| Medium | Release provenance | Release assets are attested, but there is no SBOM generation or dependency/license manifest attached to releases (`.github/workflows/release.yml:154`). | Generate CycloneDX/SPDX SBOMs for Rust/npm/Go artifacts and publish them with attestations. |
| Medium | Cargo package verification | `cargo package --workspace` without `--no-verify` currently fails on this Cargo toolchain while verifying internal workspace crates from Cargo's temporary registry (`slskr-protocol v0.0.0`: no hash listed). | Track upstream/toolchain behavior and restore full package verification once internal workspace crate verification works reliably, or add a hermetic unpack/build verification script. |
| Medium | Release tag policy | Release workflow triggers on `release-*`, not semantic `v*` tags (`.github/workflows/release.yml:10`). | Decide final tag convention and enforce it in release workflow and docs. |
| Medium | Release version metadata | Workspace crates are all versioned `0.0.0` while release artifacts derive a separate version label from tags or workflow input (`crates/slskr/Cargo.toml:3`, `.github/workflows/release.yml:26`). | Align crate/package metadata with the release version or document that crates are intentionally unpublished/internal; add a release check that artifact names, binary version, and crate metadata agree. |
| Medium | Secret scanning gate | Local `.env`, `web/.env.local`, and `.secrets/` are ignored, but CI/release has no `gitleaks`, `detect-secrets`, or equivalent secret-scanning guard. | Add a pinned secret scan to CI and release gate with allowlisted placeholders for `k8s/secrets.example.yaml` and docs. |
| Medium | OpenAPI drift | API parity work changes response shapes faster than `docs/openapi.json` and docs can track. | Add generated OpenAPI/doc drift checks to CI and fail when checked-in docs differ. |
| Medium | Compatibility smoke | slskd API compatibility smoke is opt-in because it needs external Python package install and live-style behavior (`scripts/run-release-gate.sh:55`). | Keep opt-in locally, but run it in scheduled CI with explicit secrets or hermetic fixtures. |
| Medium | Python client | Python client has no lint/type/test/audit gate, only compile coverage was run locally. | Add `ruff`, pyright or mypy, pytest smoke tests, and dependency audit. |
| Medium | Rust dependency hygiene | `cargo tree -d` shows duplicate `getrandom`, `rand`, `rand_chacha`, `rand_core`, `hashbrown`, and `thiserror` families in the release graph. | Review after dependency updates and consolidate where semver compatibility allows to reduce binary size and dependency review surface. |
| Medium | Audit tooling availability | Local audit attempts showed `cargo outdated` and `cargo udeps` are not installed, and the release gate does not provision them. | Add pinned installation or replace with CI-native dependency freshness and unused-dependency tooling such as `cargo-deny`/`cargo-machete`/scheduled outdated reports. |
| Medium | GitHub Actions supply chain | CI/release workflows use mutable action tags such as `actions/checkout@v4`, `actions/setup-node@v4`, and `softprops/action-gh-release@v2` (`.github/workflows/release.yml:44`, `.github/workflows/release.yml:175`). | Pin third-party and first-party actions to reviewed commit SHAs, automate update PRs, and document the trust policy. |
| Low | Transfer event growth | Transfer event history appends indefinitely and is recreated only when the file is absent (`crates/slskr/src/main.rs:13978`, `crates/slskr/src/main.rs:14039`). | Rotate or compact transfer event logs according to the configured transfer history limit or a byte cap. |
| Low | Rust module hygiene | `#![allow(dead_code)]` appears at crate/module level in multiple Rust modules (`crates/slskr/src/main.rs:1`, `crates/slskr/src/webhooks.rs:1`, `crates/slskr/src/routing.rs:1`). | Remove broad allowances and gate intentionally unused compatibility helpers behind tests/features. |
| Low | Deprecated npm transitive deps | Web install warns on deprecated `lodash.get`, old core-js, and Babel proposal packages. | Upgrade or replace transitive owners where practical. |
| Low | Dependency modernization | `npm outdated` shows major-version drift across web/dashboard/client-ts, including React 19, SignalR 10, date-fns 4, recharts 3, Jest 30, and TypeScript 6. | Plan compatibility upgrades separately from security fixes, with UI and generated-client regression coverage. |
| Low | Docs drift | Current docs still contain stale `localhost:8080`, `http_api_*`, `slskr:latest`, and wildcard CORS examples outside the deployment guide (`docs/http-api.md:658`, `docs/INTEGRATION_GUIDE.md:229`, `docs/http-api-features.md:272`). | Update or archive stale docs and add a grep-based docs freshness check. |

## Scans Run

- `cargo audit`
- `npm --prefix web audit --audit-level=high`
- `npm --prefix dashboard audit --audit-level=high`
- `npm --prefix client-ts audit --audit-level=high`
- `npm --prefix web audit --audit-level=moderate`
- `npm --prefix dashboard audit --audit-level=moderate`
- `npm --prefix client-ts audit --audit-level=moderate`
- `cargo metadata --format-version 1 --no-deps`
- `cargo tree -d`
- `cargo outdated --workspace` was attempted but blocked because `cargo-outdated` is not installed in this environment.
- `cargo +stable udeps --workspace --all-targets` was attempted but blocked because `cargo-udeps` is not installed in this environment.
- `npm --prefix web outdated --json`
- `npm --prefix dashboard outdated --json`
- `npm --prefix client-ts outdated --json`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `python3 -m compileall -q client-python`
- `npm --prefix web run lint`
- `npm --prefix web test`
- `npm --prefix dashboard run type-check`
- `npm --prefix dashboard run lint`
- `npm --prefix client-ts run lint`
- `npm --prefix client-ts run build`
- `kubectl kustomize k8s`
- `trivy fs --severity HIGH,CRITICAL --exit-code 1 --ignore-unfixed .` via container
- `semgrep scan --config auto --error` via container; produced findings now captured in the open burn-down.
- `scripts/check-public-posture.sh`
- `bash -n scripts/*.sh`
- `shellcheck scripts/*.sh` via container, with documented legacy-noise exclusions used by the release gate.
- `actionlint` via container.
- `scripts/diff-webui-endpoints.sh` was rerun and reported 287/291 implemented; the four reported misses are now tracked as endpoint tooling/manifest drift because conversation routes are present in the router and tests.
- Source grep passes for secrets, auth/CORS/CSRF, process execution, path handling, URL fetches, docs/deployment exposure, and frontend storage/navigation sinks.
- Focused Rust tests, formatting, clippy, shell syntax checks, and diff whitespace checks passed after the latest fixes.
- `cargo test -p slskr config_file_reader_`
- `git check-ignore -v .env web/.env.local .secrets`
- `go test ./...` was attempted in `client-go` but blocked because `go` is not installed in this environment.
