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
| High | External process launch | `/api/player/external-visualizer/launch` could spawn the configured local command whenever a command was configured. | Fixed by requiring separate `SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED=true` opt-in and recording launch/blocked/failure events. |
| Medium | Webhook registration | Webhook URL validation happened only at delivery/test time, so obviously blocked URLs could be saved. | Fixed by validating registration and admin creation URLs for scheme, host presence, localhost, direct private/link-local IP targets, and known ports. |
| Medium | Auth disabled escape hatch | `SLSKR_AUTH_DISABLED=true` could expose protected APIs on non-loopback binds without a health-surface warning. | Fixed by adding a startup warning and `/api/health` warning when auth is disabled on a non-loopback bind. |
| Low | Archive verification | `verify-release-artifacts.sh` extracted zip files without path traversal checks. | Fixed by rejecting absolute and parent-directory zip entries before extraction. |
| Low | Kubernetes secrets | Manifest references `slskr-secrets` and `grafana-admin` without templates. | Fixed by adding `k8s/secrets.example.yaml` with placeholder-only Secret manifests. |

## Open Burn-Down

| Severity | Area | Finding | Proposed fix |
| --- | --- | --- | --- |
| High | Legacy cookie auth | Backend still accepts `slskr.session` cookies for compatibility. CSRF checks reduce risk, but cookies widen the browser attack surface. | Keep for parity only if needed; otherwise add a config flag to disable cookie auth while keeping bearer/API-key auth. |
| High | CSP | Static web responses require `'unsafe-inline'` and `wasm-unsafe-eval`. | Move inline scripts/styles to bundled assets or add nonce/hash generation. |
| Medium | Webhook secret lifecycle | Webhook creation returns the secret in the response. This is acceptable as one-time display, but undocumented. | Document one-time secret return and avoid ever returning it from list/detail routes. |
| Medium | CI tooling | Local environment lacks `actionlint`, `shellcheck`, `semgrep`, and `trivy`, so the gate cannot run those classes of checks locally. | Add optional gate steps that run when installed, and CI setup that installs the chosen tools. |
| Medium | Python client | Python client has no lint/type/test/audit gate. | Add `ruff`, `mypy` or pyright, pytest smoke tests, and dependency audit. |
| Medium | Go client | Go client has no `go test`, `govulncheck`, or staticcheck gate. | Add Go CI steps and client examples compile checks. |
| Medium | Release artifacts | Release archives include the main web build but not standalone dashboard artifacts/images. | Decide whether standalone dashboard is supported as a release asset; if yes, build and publish it explicitly. |
| Medium | Docs | `docs/http-api-deployment.md` still contains stale config names, `rust:latest`, `slskr:latest`, and wildcard CORS examples. | Rewrite or archive that doc so it cannot be used as production guidance. |
| Medium | OpenAPI drift | API parity work changes response shapes faster than OpenAPI/docs can track. | Add generated OpenAPI/doc drift checks to CI and fail when checked-in docs differ. |
| Medium | Compatibility smoke | slskd API compatibility smoke is opt-in because it needs external Python package install and live-style behavior. | Keep opt-in locally, but run it in scheduled CI with explicit secrets or hermetic fixtures. |
| Medium | Rate limiting | Rate limiting keys by raw peer socket address and token digest; deployments behind proxies may collapse clients into one IP. | Add trusted proxy parsing for `Forwarded`/`X-Forwarded-For` with an explicit trusted proxy allowlist. |
| Medium | Config mutability | Several option/config compatibility routes return success without persistent mutation. | Mark no-op compatibility endpoints clearly or implement persistence with validation. |
| Medium | Path deletion parity | Download/incomplete file deletion compatibility routes currently return success stubs. | Implement scoped deletion under the download root or return explicit not-implemented status. |
| Medium | Metrics docs | Kubernetes probes and ServiceMonitor assume `/api/metrics`; metrics route exists but docs and labels are inconsistent across files. | Normalize metric path labels and update docs. |
| Low | Rust module hygiene | `#![allow(dead_code)]` appears at crate/module level in multiple Rust modules. | Remove broad allowances and gate intentionally unused compatibility helpers behind tests/features. |
| Low | Script dependencies | Release/live scripts assume Node, Python, pip, curl, tmux, sudo, network namespace tools, and WireGuard tools without a preflight summary. | Add `scripts/check-dev-tooling.sh` and call it from relevant live scripts. |
| Low | Test noise | Web tests emit repeated jsdom navigation warnings. | Mock `window.location.assign/reload` centrally in test setup. |
| Low | Deprecated npm transitive deps | Web install warns on deprecated `lodash.get`, old core-js, and Babel proposal packages. | Upgrade or replace transitive owners where practical. |
| Low | GitHub workflow lint | Workflows are valid YAML but not actionlint-verified locally. | Add actionlint to CI and local gate. |
| Low | Kubernetes NetworkPolicy | Manifests do not define ingress/egress NetworkPolicies. | Add default-deny plus explicit ingress from ingress controller and metrics scraper, and scoped egress. |
| Low | Release tag policy | Release workflow triggers on `release-*`, not semantic `v*` tags. | Decide final tag convention and document/enforce it in release docs. |

## Scans Run

- `cargo audit`
- `npm --prefix web audit --audit-level=high`
- `npm --prefix dashboard audit --audit-level=high`
- `npm --prefix client-ts audit --audit-level=high`
- `scripts/check-public-posture.sh`
- Source grep passes for secrets, auth/CORS/CSRF, process execution, path handling, URL fetches, docs/deployment exposure, and frontend storage/navigation sinks.
- `scripts/run-release-gate.sh` passed before the final Lidarr DNS-pinning fix; focused Rust/frontend checks passed after that fix.
