# Active Council Bughunt Candidate Report

This report is not a pass/fail proof. It is a fresh queue of suspicious shapes
that sit outside, or at the edge of, the current closed sweep gates. A green
all-phases council run means registered gates passed; it does not mean these
candidate lines are bugs or that no bugs exist.

Classification rule: any accepted row must be ledgered, fixed with behavior
coverage, sibling-swept, and promoted into a durable gate before closure.

## Async void boundaries

## Silent catch or lossy exception boundaries

## Callback/event invocation boundaries

## Remote/user text in diagnostics or HTTP errors

## Red-team abuse lens
docs/parity/slskdn-slsknet-runtime-parity.md:37:| Search, browse, transfer projections | Partially implemented | Download retry, resume-state preservation, rooted download-path validation, transfer progress/failure projection, peer queue-position projection, stuck-download, per-user, speed, download stats, accelerated-download compatibility reports, download batch projection, local peer transfer tests, slskd-style public vs locked/private search response projection, paged search-detail result arrays, paged/filtered search response peer groups, SQLite-backed search result restart hydration, search lifecycle update projection including caller-supplied TTL bounds, search-backed transfer alternative replacement and auto-replacement, and paged/filtered slskd-shaped user browse/directory projections are covered. Remaining work: broader browse/search parity plus optional live interop. |
docs/parity/slskdn-slsknet-runtime-parity.md:41:| Wishlist interval scheduling | Implemented/needs-live-proof | Client scheduler primitive honors server `WishlistInterval` with positive minimum interval validation; daemon session manager now snapshots wishlist terms, records scheduled wishlist searches, and emits `WishlistSearch` while connected. Remaining work: optional live interop proof. |
docs/parity/slskdn-slsknet-runtime-parity.md:42:| Security posture | Gate-backed/needs-live-proof | Bind/public-posture checks, WebSocket auth coverage, CSP, OAuth callback state isolation, webhook outbound policy, rate-limit proxy policy, storage/transfer pressure gates, secret scanning, protocol scalar/adversarial gates, and remediation baseline are implemented. Remaining work: optional live exposure/interoperability proof in deployment environments. |
docs/parity/slskdn-slsknet-runtime-parity.md:55:- `compat-ack`: route or workflow intentionally returns a compatibility shell.
docs/parity/slskdn-slsknet-runtime-parity.md:70:| Store-backed compatibility projections | Collection, wishlist, contact, share, and bridge helper routes expose local state | Daemon API tests cover `/api/shared`, `/api/contacts/nearby`, collection item update/delete/reorder, wishlist update/delete, and `/api/bridge/transfer/:id/progress` projecting existing local stores instead of fixed compatibility shells. |
docs/parity/slskdn-slsknet-runtime-parity.md:71:| Activity and recommendation projections | Interests, now-playing, source-feed, and bridge admin compatibility routes expose local state | Daemon API tests cover user interest projection, interest-backed Soulseek recommendations, item recommendation/similar-user projections, now-playing POST/GET, source-feed preview/feed projection, and bridge admin stats deriving transfer totals instead of fixed empty compatibility shells. |
docs/parity/slskdn-slsknet-runtime-parity.md:73:| System mutation compatibility projections | Admin/config, application runtime, profile, bridge, relay, wishlist import, and share grant helper routes expose local state | Daemon API tests cover admin stats deriving transfer/search totals, plugin/config summaries from integration config, application restart and GC compatibility state, profile updates mutating the session projection, profile invite/cache warm/backfill/SongID/Lidarr operation counters surfacing through runtime state, bridge start/stop/config updates surfacing through bridge status and application runtime state, autoreplace/preferences state, relay/relay-agent enable-disable state, CSV wishlist import creating items, and share-grant token/backfill helpers deriving results from grant and collection stores with persisted/local status for existing grants. |
docs/parity/slskdn-slsknet-runtime-parity.md:75:| Inventory and operations projections | Hash DB, backfill, mesh sync, logs, events, relay controller, and batch routes derive from local runtime state | Daemon API tests cover hash entries derived from share index records, backfill candidates/stats derived from search and share stores, hash backfill queue counts, mesh sync status derived from watched/capable peers, event mutation responses and logs projected from the event store, relay controller token acknowledgements carrying relay/share state, and limited local batch execution for health/config/capabilities/stats reads. |
docs/parity/slskdn-slsknet-runtime-parity.md:76:| Media and status projections | Profile, source-feed creation, SongID, KPI, pod, federation, Solid, and security status routes expose local state | Daemon API tests cover profile data from session state, source-feed creation seeding wishlist-backed feeds, SongID runs/details/matrices deriving matches from library and share stores, KPI summaries from transfers/searches/shares, pod and federation status from room/user/mesh state, Solid status from collections/grants, and security dashboard counts from watched peers/events/webhooks. |
docs/parity/slskdn-slsknet-runtime-parity.md:77:| Native service compatibility projections | slskdN roots, PodCore, streams, rooms, mesh health, playback, trace, fairness/ranking, port-forwarding, signal, backfill, federation, Solid, and security ban routes expose local state | Daemon API tests cover slskdN summaries from session/share/search/transfer/user/room/library stores, slskdN library health from library issues, PodCore content search from the share index, room ticker/member compatibility mutations updating joined room state, local stream availability from shares/transfers, peer-stream and mesh-stream preview ticket routes, security bans mutating local state and dashboard counts, fairness/ranking rows from watched users/searches/transfers, port-forwarding rows from listener bind state, federation rows from watched/capable peers, Solid rows from collections, specialized native service status routes, and the native compatibility fallback returning family-specific local counts/items/jobs instead of generic disabled shells. |
docs/parity/slskdn-slsknet-runtime-parity.md:81:| Transfers | Retryable failed downloads, rooted remote-path acceptance, terminal details before failure | Transfer API and local peer tests cover explicit failed-download retry, stale reason clearing, preserved byte count for resume, scoped download paths, progress events, peer queue-position projection, stuck-download, per-user, speed, download stats, and accelerated-download compatibility reports, search-backed alternative source replacement and auto-replacement, local peer upload/download execution, and rejection paths. Remaining gate: live interop. |
docs/parity/slskdn-slsknet-runtime-parity.md:82:| Obfuscation | Plain/obfuscated `P`, `D`, `F` fallback matrix | Loopback tests cover obfuscated `P`/`D`/`F` demux, plain rejection of obfuscated init, obfuscated rejection of plain init, preferred obfuscated file-transfer execution, and fallback to plain `P`/`F` when advertised obfuscation fails. |
docs/parity/slskdn-slsknet-runtime-parity.md:83:| Security hardening | Bind exposure validation, feature gates, path/SSRF/logging checks | `scripts/check-remediation-baseline.sh` covers endpoint drift, browser token persistence, unsafe blank opens, WebSocket auth, CSP, webhook outbound policy, rate-limit proxy policy, storage listing pressure, transfer event growth, workflow/release policy, package matrix, dependency hygiene, release metadata, secret scanning, SDK gates, audit tooling, module hygiene, docs drift/freshness, council gates, protocol taint/adversarial checks, shell hygiene, Kubernetes public posture, and compatibility no-op documentation. |
docs/parity/slskdn-slsknet-runtime-parity.md:84:| UI performance parity | Implemented | Rust WebUI tests assert a single active route owns live probes/workspace data, hidden RustyMilk starts stopped, diagnostics/workspaces advertise lazy state, and initial shell markup has no hidden-pane polling loops. |
docs/rust-web-ui.md:12:- Browser shell: `crates/slskr-web/static/index.html`
docs/rust-web-ui.md:17:The first Rust shell covers the full current route/navigation inventory and the
docs/rust-web-ui.md:18:major API-backed surfaces: application state, session control, search, wishlist,
docs/rust-web-ui.md:20:system status. The shell owns route page rendering, active nav state, History
docs/slskr.config.example.toml:14:# Prefer a secret manager or environment variables for real credentials.
docs/slskr.config.example.toml:16:# password = "pass"
docs/slskr.config.example.toml:47:# Loopback-only binds default to no API auth unless api_token is configured.
docs/slskr.config.example.toml:48:# Non-loopback binds require an API token unless auth is explicitly disabled.
docs/slskr.config.example.toml:50:api_token = "replace-with-a-random-token"
docs/slskr.config.example.toml:51:cookie_auth_enabled = false
docs/slskr.config.example.toml:54:# Trust forwarded client IP headers only from these proxy CIDRs.
docs/slskr.config.example.toml:55:trusted_proxy_cidrs = ["127.0.0.1/32"]
docs/slskr.config.example.toml:62:# Configure a Spotify app client ID and redirect URI, then the WebUI can open the
docs/slskr.config.example.toml:63:# returned authorization_url from /api/integrations/spotify/authorize.
docs/slskr.config.example.toml:66:# client_secret = "optional-server-side-secret"
docs/slskr.config.example.toml:67:# redirect_uri = "http://127.0.0.1:5030/api/integrations/spotify/callback"
docs/slskr.config.example.toml:75:# url = "http://127.0.0.1:8686"
docs/slskr.config.example.toml:76:# api_key = "lidarr-api-key"
examples/README.md:71:TOKEN="your-bearer-token"
examples/README.md:72:API="http://localhost:8080"
examples/README.md:81:     -H "Origin: http://localhost:8080" \
examples/README.md:84:         {"id":"d1","method":"POST","path":"/api/transfers","body":{"direction":"download","peer_username":"user1","filename":"song.mp3"}},
examples/README.md:85:         {"id":"d2","method":"POST","path":"/api/transfers","body":{"direction":"download","peer_username":"user2","filename":"album.zip"}}
examples/README.md:108:  baseUrl: 'http://localhost:8080',
examples/README.md:109:  token: 'your-token'
examples/README.md:143:TOKEN="your-bearer-token"
examples/README.md:144:API="http://localhost:8080"
examples/README.md:156:     -H "Origin: http://localhost:8080" \
examples/README.md:163:     -H "Origin: http://localhost:8080" \
examples/README.md:184:  <script src="https://cdn.jsdelivr.net/npm/@slskr/api-client"></script>
examples/README.md:195:      baseUrl: 'http://localhost:8080',
examples/README.md:196:      token: 'your-token'
examples/README.md:205:        transfers.map(t => `${t.filename}: ${t.progress_percent}%`).join('<br>');
examples/README.md:233:    baseUrl: 'http://localhost:8080',
examples/README.md:234:    token: 'token'
examples/README.md:250:      .execute();
examples/README.md:277:    baseUrl: 'http://localhost:8080',
examples/README.md:278:    token: 'token',
examples/README.md:287:      filename: 'file.mp3'
examples/README.md:294:        console.error('Invalid token - please authenticate');
examples/README.md:314:- slskr server running on http://localhost:8080
examples/README.md:318:1. Set Bearer token:
examples/README.md:320:export SOULSEEK_TOKEN="your-bearer-token"
examples/README.md:357:      - HTTP_API_BEARER_TOKEN=secret-token
examples/README.md:369:      - SOULSEEK_API_URL=http://slskr:8080
examples/README.md:370:      - SOULSEEK_TOKEN=secret-token
examples/README.md:381:  SOULSEEK_API_URL: "http://slskr:8080"
examples/README.md:382:  SOULSEEK_TOKEN: "secret-token"
scripts/run-council-active-bughunt.sh:25:    rg -n -U --with-filename --pcre2 --hidden --glob '!.git/**' --glob '!.council/**' "$pattern" "$@" || true
scripts/run-council-active-bughunt.sh:41:# Replace paths and patterns for your repo. Add narrow sections whenever a
scripts/run-council-active-bughunt.sh:61:  '(log|logger|Diagnostic|Console\.WriteLine|StatusCode\(|BadRequest\()[^;\n]*(username|query|filename|directory|token|message)' \
scripts/run-council-active-bughunt.sh:66:  '(token|secret|password|authorization|cookie|api[-_]?key|session|redirect|proxy|forwarded|path|filename|exec|spawn|shell|http://|https://)' \
scripts/check-council-sweep-counts.sh:82:#   "secret-pattern sweep count matches scanner"
scripts/scan-bug-council-candidates.sh:24:  rg -n --with-filename --pcre2 --hidden --glob '!.git/**' "$pattern" "$@" || true
scripts/scan-bug-council-candidates.sh:33:  'PRIVATE KEY|gh[pousr]_[A-Za-z0-9_]{36,}|xox[baprs]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|(?i)(api[_-]?key|access[_-]?token|client[_-]?secret)' \
scripts/scan-bug-council-candidates.sh:57:#   'tokio::spawn|select!|timeout\(|sleep\(|interval\(|mpsc|broadcast|oneshot' \
scripts/check-local-identity-leaks.sh:17:tmp_tokens="$(mktemp)"
scripts/check-local-identity-leaks.sh:20:trap 'rm -f "$tmp_tokens" "$tmp_commits" "$tmp_files"' EXIT
scripts/check-local-identity-leaks.sh:22:add_token() {
scripts/check-local-identity-leaks.sh:23:  local token="$1"
scripts/check-local-identity-leaks.sh:24:  token="${token//$'\n'/}"
scripts/check-local-identity-leaks.sh:25:  token="${token//$'\r'/}"
scripts/check-local-identity-leaks.sh:26:  [[ ${#token} -ge 3 ]] || return 0
scripts/check-local-identity-leaks.sh:27:  case "$token" in
scripts/check-local-identity-leaks.sh:32:  printf '%s\n' "$token" >>"$tmp_tokens"
scripts/check-local-identity-leaks.sh:35:add_token "${LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:36:add_token "${SLSKDN_LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:37:add_token "${SLSKDN_FORBIDDEN_LOCAL_HOSTNAME:-}"
scripts/check-local-identity-leaks.sh:38:add_token "$(hostname -s 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:39:add_token "${USER:-}"
scripts/check-local-identity-leaks.sh:40:add_token "$(id -un 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:41:add_token "$(basename "${HOME:-}" 2>/dev/null || true)"
scripts/check-local-identity-leaks.sh:43:read_csv_tokens() {
scripts/check-local-identity-leaks.sh:46:  IFS=',' read -ra tokens <<<"$value"
scripts/check-local-identity-leaks.sh:47:  for token in "${tokens[@]}"; do
scripts/check-local-identity-leaks.sh:48:    add_token "$token"
scripts/check-local-identity-leaks.sh:52:read_csv_tokens "${LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:53:read_csv_tokens "${SLSKDN_LOCAL_IDENTITY_DENYLIST:-}"
scripts/check-local-identity-leaks.sh:58:  while IFS= read -r token; do
scripts/check-local-identity-leaks.sh:59:    [[ "$token" =~ ^[[:space:]]*# ]] && continue
scripts/check-local-identity-leaks.sh:60:    add_token "$token"
scripts/check-local-identity-leaks.sh:67:sort -u "$tmp_tokens" -o "$tmp_tokens"
scripts/check-local-identity-leaks.sh:68:if [[ ! -s "$tmp_tokens" ]]; then
scripts/check-local-identity-leaks.sh:69:  echo "No local identity tokens configured for scanning."
scripts/check-local-identity-leaks.sh:77:  local path="$2"
scripts/check-local-identity-leaks.sh:78:  local display_path="${3:-$path}"
scripts/check-local-identity-leaks.sh:81:  [[ -f "$path" ]] || return 0
scripts/check-local-identity-leaks.sh:83:    rg --json --fixed-strings --ignore-case --file "$tmp_tokens" "$path" |
scripts/check-local-identity-leaks.sh:84:      jq -r --arg label "$label" --arg display_path "$display_path" 'select(.type == "match") | "\($label): \($display_path):\(.data.line_number)"' |
scripts/check-local-identity-leaks.sh:96:  trap 'rm -f "$tmp_tokens" "$tmp_commits" "$tmp_files" "$tmp_unreleased"' EXIT
scripts/check-local-identity-leaks.sh:117:  -path './.git' -prune -o \
scripts/check-local-identity-leaks.sh:118:  -path './node_modules' -prune -o \
scripts/check-local-identity-leaks.sh:119:  -path './vendor' -prune -o \
scripts/check-local-identity-leaks.sh:120:  -path './target' -prune -o \
scripts/check-local-identity-leaks.sh:121:  -path './dist' -prune -o \
scripts/check-local-identity-leaks.sh:122:  -path './build' -prune -o \
scripts/check-local-identity-leaks.sh:123:  -path './zeek/pkg' -prune -o \
scripts/check-local-identity-leaks.sh:125:    -path './.github/release-notes/*' -o \
scripts/check-local-identity-leaks.sh:126:    -path './docs/dev/release-copy.md' -o \
scripts/check-local-identity-leaks.sh:127:    -path './docs/release*.md' -o \
scripts/check-local-identity-leaks.sh:128:    -path './docs/RELEASE*.md' -o \
scripts/check-local-identity-leaks.sh:129:    -path './packaging/winget/*' \
scripts/check-local-identity-leaks.sh:132:while IFS= read -r path; do
scripts/check-local-identity-leaks.sh:133:  [[ -n "$path" ]] || continue
scripts/check-local-identity-leaks.sh:134:  check_file "$path" "$path"
docs/security-bug-burndown.md:14:| Medium | Dashboard auth | Standalone dashboard persisted API keys in `localStorage`. | Fixed by keeping API keys in `sessionStorage`; API URL remains persistent. |
docs/security-bug-burndown.md:15:| Medium | Client logging | TypeScript client debug logging printed request bodies, which can include secrets. | Fixed with recursive secret-field redaction. |
docs/security-bug-burndown.md:16:| Medium | Test script auth | slskd API compatibility smoke had a deterministic default API token. | Fixed by requiring `SLSKR_SLSKD_API_SMOKE_TOKEN`. |
docs/security-bug-burndown.md:17:| Medium | API auth bootstrap | `/api/session/enabled` could require auth even though login bootstrap calls it before a token exists. | Fixed by making it explicitly public and adding route coverage. |
docs/security-bug-burndown.md:18:| Medium | Frontend session handling | Web session check masked network failures with a secondary `error.response.status` TypeError. | Fixed with optional response handling and a regression test. |
docs/security-bug-burndown.md:21:| Low | Docs | Security docs said the dashboard saves API tokens in a cookie, which no longer describes the preferred browser behavior. | Fixed to document bearer/session-storage behavior and legacy cookie compatibility. |
docs/security-bug-burndown.md:24:| High | Kubernetes runtime | `slskr-api` ran three replicas against a daemon that owns a live Soulseek session and local in-memory/session state. | Fixed default manifests to run one API replica and constrained the API HPA to one replica. |
docs/security-bug-burndown.md:26:| High | Browser token persistence | Main React web UI supported `rememberMe` bearer-token persistence in `localStorage`. | Fixed by removing the login persistence toggle, storing login tokens in `sessionStorage`, and ignoring legacy persistent tokens. |
docs/security-bug-burndown.md:27:| High | Legacy cookie auth | Backend accepted `slskr.session` cookies whenever API-token auth was enabled. | Fixed by adding `SLSKR_API_COOKIE_AUTH_ENABLED` / `[auth].cookie_auth_enabled`, defaulting legacy cookie auth off while preserving an explicit compatibility opt-in. |
docs/security-bug-burndown.md:28:| High | External process launch | `/api/player/external-visualizer/launch` could spawn the configured local command whenever a command was configured. | Fixed by requiring separate `SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED=true` opt-in and recording launch/blocked/failure events. |
docs/security-bug-burndown.md:30:| Medium | Webhook secret lifecycle | Webhook creation returned the secret without documenting that it is a one-time creation-only value. | Fixed by adding `docs/WEBHOOK_API.md`, marking create responses with `secretReturnedOnce`, and documenting that list/detail/log routes omit secrets. |
docs/security-bug-burndown.md:32:| Medium | Path deletion parity | Download/incomplete file deletion compatibility routes returned success stubs instead of scoped deletion. | Fixed by decoding slskd path segments, rejecting traversal/absolute paths/symlinks, and deleting only under the downloads or incomplete storage root. |
docs/security-bug-burndown.md:33:| Medium | Path listing parity | Download/incomplete directory compatibility routes returned empty shells and recursive listings had no traversal budget. | Fixed by listing scoped storage roots, rejecting traversal/symlink escapes, supporting recursive responses, and capping listings at 16,384 entries. |
docs/security-bug-burndown.md:36:| Medium | Config compatibility endpoints | `/api/options/yaml/location` returned a fake `/etc/slskr/config.yaml` path and YAML upload/validate routes accepted non-string JSON bodies. | Fixed by reporting the actual loaded config path or `runtime://memory`, returning read-only TOML compatibility text, and rejecting non-string upload/validate payloads. |
docs/security-bug-burndown.md:38:| Low | Archive verification | `verify-release-artifacts.sh` extracted zip files without path traversal checks. | Fixed by rejecting absolute and parent-directory zip entries before extraction. |
docs/security-bug-burndown.md:39:| Low | Kubernetes secrets | Manifest references `slskr-secrets` and `grafana-admin` without templates. | Fixed by adding `k8s/secrets.example.yaml` with placeholder-only Secret manifests. |
docs/security-bug-burndown.md:40:| Medium | CI tooling | Local gate lacked workflow/shell/security tool hooks beyond Rust/npm advisory checks. | Fixed by adding optional local `shellcheck`, `actionlint`, `semgrep`, and `trivy` release-gate steps plus CI setup for shellcheck/actionlint. |
docs/security-bug-burndown.md:41:| Medium | Docs | `docs/http-api-deployment.md` contained stale config names, `rust:latest`, `slskr:latest`, and wildcard CORS examples. | Fixed by replacing it with current `SLSKR_*` config, reverse-proxy, Kubernetes, metrics, and no-wildcard-CORS guidance. |
docs/security-bug-burndown.md:42:| Medium | Metrics docs | Metrics path guidance was split between `/api/metrics` and `/api/v0/metrics`. | Fixed deployment docs to state both aliases and that Kubernetes scrape config uses `/api/metrics`. |
docs/security-bug-burndown.md:43:| High | Kubernetes metrics | ServiceMonitor scraped protected `/api/metrics` without authentication and referenced an unused metrics service port. | Fixed by scraping the real `http` port with `SLSKR_API_TOKEN` from `slskr-secrets`, removing unauthenticated scrape annotations, and dropping the unused metrics port/config. |
docs/security-bug-burndown.md:45:| Low | Built-in dashboard token field | The embedded fallback dashboard rendered the API token input as a text field. | Fixed by using a password input with token-safe browser attributes. |
docs/security-bug-burndown.md:47:| Low | Config file type | Config loading capped config file size but did not reject non-regular paths before reading. | Fixed by requiring regular files and adding directory rejection coverage. |
docs/security-bug-burndown.md:50:| Medium | Webhook secrets | Webhook creation accepted caller-supplied signing secrets without minimum strength checks. | Fixed by requiring supplied secrets to be at least 32 bytes, printable, and have basic character variety on public/admin creation routes while preserving generated secrets by default. |
docs/security-bug-burndown.md:51:| Medium | Frontend auth passthrough | `session.authHeaders()` emitted `Authorization: Bearer n/a` in passthrough mode. | Fixed by omitting Authorization for passthrough tokens and adding regression coverage. |
docs/security-bug-burndown.md:55:| Medium | TypeScript client path escaping | The TypeScript client interpolated IDs, usernames, and room names directly into URL paths. | Fixed by encoding all client-composed dynamic path segments with `encodeURIComponent`. |
docs/security-bug-burndown.md:56:| Medium | Python client path escaping | The Python client interpolated path segments and used `urljoin`, allowing caller-controlled path rewriting. | Fixed by encoding dynamic path segments with `quote(..., safe="")` and joining against the configured base URL directly. |
docs/security-bug-burndown.md:57:| Medium | Python WebSocket lifecycle | Python WebSocket connect created an anonymous `aiohttp.ClientSession` and retained only the WebSocket response. | Fixed by storing and closing the session on disconnect and failed connect attempts. |
docs/security-bug-burndown.md:59:| Low | Go client URL escaping | Go client methods interpolated usernames, message IDs, and room IDs directly into paths. | Fixed by escaping path parameters with `url.PathEscape`. |
docs/security-bug-burndown.md:60:| Low | Client error redaction | Go client errors included raw response bodies that could echo upstream secrets. | Fixed by redacting common secret fields from JSON and text error bodies before returning errors. |
docs/security-bug-burndown.md:61:| Medium | Frontend prototype pollution | Adversarial settings used dynamic nested object writes and only guarded two of the array/object update helpers. | Fixed by rejecting `__proto__`, `constructor`, and `prototype` paths in all nested setting update helpers. |
docs/security-bug-burndown.md:63:| Low | Config secret permissions | TOML config files can contain credentials and API tokens without warning when group/world-readable. | Fixed by warning on Unix when a config file with known secret fields has group/other permission bits set. |
docs/security-bug-burndown.md:64:| Medium | Webhook delivery DoS | `/api/webhooks/:id/test` could spawn unbounded delivery tasks. | Fixed by sharing a bounded webhook delivery semaphore and returning `429` when the delivery pool is full. |
docs/security-bug-burndown.md:66:| Medium | Webhook dispatch concurrency | Normal webhook event dispatch spawned per-webhook delivery tasks without acquiring the shared delivery pool. | Fixed by passing the shared webhook delivery semaphore into normal dispatch and dropping over-capacity deliveries before outbound work starts. |
docs/security-bug-burndown.md:68:| Medium | Rate limiting | Anonymous rate limiting keyed every reverse-proxied client by the proxy socket address. | Fixed by adding trusted proxy CIDR configuration, parsing `Forwarded` and `X-Forwarded-For` only from allowlisted proxies, and adding spoofing rejection tests/gate coverage. |
docs/security-bug-burndown.md:71:| Medium | Compatibility no-op inventory | Preserved slskd parity routes returned empty shells or compatibility acknowledgements without a complete tested inventory. | Fixed by documenting logs/cache/bridge/config/bans/share-grant token/backfill and MusicBrainz subscription shells, adding explicit acknowledgement status where useful, and asserting shapes in tests/gates. |
docs/security-bug-burndown.md:72:| High | Release workflow | The release version step interpolated GitHub context expressions directly inside a shell script. | Fixed by passing workflow context through `env:` and quoting shell variables. |
docs/security-bug-burndown.md:75:| Low | Kubernetes hardening | API pods omitted `runAsGroup`, `seccompProfile`, and disabled service-account-token automounting. | Fixed by setting restricted pod/container security context fields and `automountServiceAccountToken: false`. |
docs/security-bug-burndown.md:77:| Low | Tar archive verification | Release artifact verification extracted tar archives without member safety checks. | Fixed by validating tar paths, rejecting links/special files, and extracting only after all members pass. |
docs/security-bug-burndown.md:81:| Medium | Browser token persistence | ListenBrainz user tokens were saved in persistent `localStorage`. | Fixed by storing ListenBrainz tokens in `sessionStorage` and updating UI regression coverage. |
docs/security-bug-burndown.md:82:| Medium | Endpoint parity tooling drift | Endpoint parity tooling reported implemented conversation routes as missing and included malformed `GET /conversations:var`. | Fixed by removing the malformed manifest entry and teaching the checker about `path_segment_after` dynamic handlers. |
docs/security-bug-burndown.md:85:| Medium | External metadata privacy | Lyrics lookup could persist open state and automatically send current artist/title metadata to LRCLIB later. | Fixed by making lyrics lookup a per-session explicit action and ignoring stale persisted lyrics-open state. |
docs/security-bug-burndown.md:88:| Low | Test noise | Web tests emitted repeated jsdom navigation warnings from the unauthorized reload path. | Fixed by skipping page reload only under Vitest `MODE=test` and adding regression coverage for test and production behavior. |
docs/security-bug-burndown.md:89:| High | Browser WebSocket auth | Browser clients opened `/api/events/ws` without a bearer-capable auth mechanism. | Fixed by accepting a `slskr.api-token.<percent-encoded-token>` WebSocket subprotocol on the server and using it from the TypeScript SDK. |
docs/security-bug-burndown.md:90:| High | Main web event feed auth | The React web event hub opened `/api/events/ws` without a browser-safe token path. | Fixed by sending the same auth subprotocol for session bearer tokens, omitting passthrough/missing tokens, and adding Vitest coverage. |
docs/security-bug-burndown.md:92:| High | CSP | Static and generic responses used broad inline script/style allowances, and the Rust WASM shell used an inline module bootstrap. | Fixed by adding strict or nonce-backed generic CSP, moving the Rust WASM bootstrap to a static module, rejecting broad inline allowances for static web assets, and scoping `wasm-unsafe-eval` only to Rust WASM builds. |
docs/security-bug-burndown.md:97:| Medium | Secret scanning gate | Local `.env`, `web/.env.local`, and `.secrets/` are ignored, but tracked files lacked a reproducible committed-secret guard. | Fixed by adding `scripts/check-secret-scanning.sh` to verify ignored local secret paths and scan tracked files for private-key blocks and high-entropy secret-like assignments. |
docs/security-bug-burndown.md:98:| Medium | Python client | Python client had no lint/test/dependency gate beyond compile/import coverage. | Fixed by adding `scripts/check-python-client-quality.sh`, pytest smoke tests for the client helpers, and SDK-gate execution of dev install, pytest, import, and `pip check`. |
docs/security-bug-burndown.md:104:| Medium | Client transfer chunk allocation | The reusable client transfer helper could allocate a caller-supplied remaining length in one `Vec`. | Fixed by adding `DEFAULT_MAX_TRANSFER_CHUNK_LEN`, rejecting oversized chunk reads before allocation, and covering direct chunk and `receive_file_from` paths. |
docs/security-bug-burndown.md:105:| Medium | Protocol scalar narrowing | API routes narrowed oversized JSON tokens/message IDs into protocol `u32` values with `as`. | Fixed by rejecting out-of-range search response tokens and message acknowledgement IDs before protocol command emission, with regression tests. |
docs/security-bug-burndown.md:108:| Medium | TypeScript abort timer cleanup | TypeScript SDK request timers were cleared only after `fetch` resolved, so rejected requests could leave stale abort timers alive during retries. | Fixed by clearing timers in `finally`, adding a Jest regression, and adding TS test/build execution to the SDK gate. |
docs/security-bug-burndown.md:110:| Medium | WebSocket auth examples | Raw WebSocket docs showed unauthenticated browser snippets and a Node constructor/header pattern that did not match the enforced subprotocol auth path. | Fixed by updating raw browser and Node examples to pass the supported `slskr.api-token.<encoded-token>` WebSocket subprotocol. |
docs/security-bug-burndown.md:118:| Medium | OpenAPI drift | Runtime `/api/openapi.json` and checked-in `docs/openapi.json` could drift because they were generated through separate paths. | Fixed by serving the checked-in OpenAPI spec at runtime, packaging an identical crate-local OpenAPI copy, adding an equality regression test, and strengthening the OpenAPI drift gate. |
docs/security-bug-burndown.md:137:- `scripts/check-secret-scanning.sh`
docs/security-bug-burndown.md:163:- `shellcheck scripts/*.sh` via container, with documented legacy-noise exclusions used by the release gate.
docs/security-bug-burndown.md:166:- Source grep passes for secrets, auth/CORS/CSRF, process execution, path handling, URL fetches, docs/deployment exposure, and frontend storage/navigation sinks.
docs/security-bug-burndown.md:167:- Focused Rust tests, formatting, clippy, shell syntax checks, and diff whitespace checks passed after the latest fixes.
docs/security-bug-burndown.md:169:- `git check-ignore -v .env web/.env.local .secrets`
docs/dev/bug-burndown-ledger.md:18:| Adversarial Reviewer | Abuse cases, confused-deputy paths, SSRF, secret exposure, persistence surprises, and bypass attempts. |
docs/dev/bug-burndown-ledger.md:39:| BUG-001 | Backend/API | CSP | High | High | Generic responses now use strict or nonce-backed CSP, static web responses reject `'unsafe-inline'`, and `wasm-unsafe-eval` is scoped only to the Rust WASM shell when `slskr_web.wasm` is present. | XSS blast radius is reduced for bundled UIs while preserving documented Rust/WASM startup requirements. | Backend/Security + Rust Web UI | CSP tests reject broad inline permissions for non-WASM shells, the Rust WASM shell uses an external bootstrap module, and `scripts/check-csp-policy.sh` enforces the scoped exception. | Verified |
docs/dev/bug-burndown-ledger.md:41:| BUG-003 | Client SDKs | WebSocket auth | High | High | `client-ts/src/websocket-client.ts` now opens `/api/events/ws` with a `slskr.api-token.<percent-encoded-token>` WebSocket subprotocol, and the server parses that protocol into bearer authorization before route auth. | Browser TypeScript clients can authenticate event streams in token-auth deployments without unsupported custom WebSocket headers. | Client SDKs + Backend/Security | Authenticated browser-safe event-stream subprotocol tests in SDK/static gate and server route auth. | Verified |
docs/dev/bug-burndown-ledger.md:42:| BUG-004 | React Web UI | WebSocket auth | High | High | `web/src/lib/hubFactory.js` now sends the same browser-safe auth subprotocol for session bearer tokens and omits it for passthrough/missing tokens, with Vitest coverage. | Main web event feed authenticates in token-auth deployments without leaking legacy persistent tokens or passthrough sentinels. | Frontend/API Handling + Backend/Security | Authenticated React event-feed unit coverage using the chosen browser-safe auth path. | Verified |
docs/dev/bug-burndown-ledger.md:46:| BUG-008 | Backend/API | Rate limiting | Medium | Medium | Anonymous rate limiting now uses `Forwarded` or `X-Forwarded-For` only when the immediate peer matches `SLSKR_TRUSTED_PROXY_CIDRS` / `[auth].trusted_proxy_cidrs`; untrusted peers keep the raw socket address. | Reverse-proxy deployments can key anonymous clients separately without accepting spoofed forwarded headers from direct clients. | Backend/Security | Trusted proxy parsing tests with explicit allowlist and spoofing rejection cases plus `scripts/check-rate-limit-proxy-policy.sh`. | Verified |
docs/dev/bug-burndown-ledger.md:49:| BUG-011 | Docs/Config | Compatibility no-op inventory | Medium | High | Preserved parity routes for logs/cache/bridge/config/bans/share-grant token/backfill and MusicBrainz subscriptions are documented as empty shells or `compatibility_acknowledgement` responses, with route tests asserting the advertised shape. | Compatibility endpoints no longer look fully supported when they are intentional no-ops or empty capability shells. | Docs/Config + Backend/Security | Inventory each compatibility acknowledgement/empty shell in docs/OpenAPI and assert the advertised shape in tests. | Verified |
docs/dev/bug-burndown-ledger.md:50:| BUG-012 | Release/Ops | Release SBOM | Medium | High | Release workflow now generates `slskr-cyclonedx.json` and `slskr-dependency-manifest.json` from checked-in Rust/npm/Go/Python package metadata, includes them in `SHA256SUMS.txt`, publishes them as release assets, and attests `release/*.json`. | Consumers can audit release dependencies from published assets alongside checksums and provenance attestations. | Release/Ops | `scripts/check-package-artifact-matrix.sh` requires the generator, manifest names, JSON attestation path, and verified ledger status. | Verified |
docs/dev/bug-burndown-ledger.md:51:| BUG-013 | Release/Ops | Cargo package verification | Medium | Medium | `scripts/check-release-package.sh` packages the runtime crates explicitly, then runs `scripts/verify-cargo-package-contents.sh` to inspect each `.crate`, require every tracked crate input, unpack packages into a temporary workspace, restore packaged `Cargo.toml.orig` path dependencies, and run `cargo check --workspace` from the unpacked inputs. | Missing package inputs are caught while avoiding non-publishable WASM migration dependencies in the Cargo package gate. | Release/Ops | Package gate performs archive content verification plus unpacked workspace build verification, and `scripts/check-package-artifact-matrix.sh` keeps the gate registered. | Verified |
docs/dev/bug-burndown-ledger.md:52:| BUG-014 | Release/Ops | Release tag policy | Medium | High | Release workflow now triggers only on `release-v*`, validates tag pushes against `release-v<semver>`, publishes only `refs/tags/release-v...`, and documents that plain `v*` or loose `release-*` tags are not release triggers. | Operators have one explicit release tag convention and malformed release tags fail before builds. | Release/Ops | `scripts/check-workflow-release-policy.sh` enforces the `release-v<semver>` docs, trigger, publish condition, and validation tokens. | Verified |
docs/dev/bug-burndown-ledger.md:54:| BUG-016 | Release/Ops | Secret scanning | Medium | High | `scripts/check-secret-scanning.sh` verifies ignored local secret paths stay ignored and scans tracked files for private-key blocks and high-entropy secret-like assignments with placeholder/example allowlisting. | Accidental secret commits now fail the remediation baseline and release gate before packaging. | Release/Ops + Adversarial Reviewer | Pinned secret scanner in CI/release with placeholder allowlist coverage. | Verified |
docs/dev/bug-burndown-ledger.md:56:| BUG-018 | Tests/Tooling | Compatibility smoke coverage | Medium | Medium | `.github/workflows/live-parity.yml` now runs the Rust web UI headless audit plus the hermetic local `slskd_api` automation compatibility smoke on a weekly schedule and manual dispatch, uploading screenshots, Rust web assets, and daemon logs as artifacts. It also runs the credentialed public live interop matrix when `SLSKR_LIVE_INTEROP_ENV` is configured, or uploads an explicit skipped TSV when credentials are absent. | Compatibility regressions now have a scheduled CI signal without forcing every local release gate to install external Python packages, and public live evidence is packaged whenever credentials are available. | Tests/Tooling | `scripts/check-workflow-release-policy.sh` requires the live parity workflow, schedule, Rust UI audit, slskd API smoke, credentialed live interop job, token envs, and artifact paths. | Verified |
docs/dev/bug-burndown-ledger.md:57:| BUG-019 | Client SDKs | Python client gate | Medium | High | `scripts/check-python-client-quality.sh` enforces Python SDK lint/version policy, `client-python/tests/test_client.py` covers URL/path escaping, batch helpers, WebSocket URL/topic behavior, public exports, and `scripts/check-client-sdk-gates.sh` installs the dev extra, runs pytest smoke tests, and checks installed dependencies. | Python SDK regressions now fail the SDK gate instead of shipping with import-only coverage. | Client SDKs | Add lint/type/test/audit coverage, pytest smoke tests, and dependency consistency checks. | Verified |
docs/dev/bug-burndown-ledger.md:72:| BUG-034 | Backend/API | Protocol scalar narrowing | Medium | High | API routes narrowed JSON/search/message IDs into Soulseek protocol `u32` values with `as`, allowing oversized client-supplied values to wrap before protocol command emission. Search response tokens and message acknowledgement IDs now use checked `u32::try_from` and return `400` when out of range. | API inputs can no longer wrap into unrelated protocol tokens or acknowledgement IDs. | Backend/API + Network Runtime | Oversized token/ack route tests plus protocol scalar inventory classification. | Verified |
docs/dev/bug-burndown-ledger.md:74:| BUG-036 | Client SDKs | SDK connect timeout | Medium | High | Public SDK helpers `ServerConnection::connect`, `connect_peer_messages`, `connect_distributed`, and `connect_file_transfer` delegated directly to `TcpStream::connect`, so consumers not wrapping them could hang indefinitely on slow network paths. Defaults now use `DEFAULT_CONNECT_TIMEOUT` and timeout-specific variants are available. | SDK consumers get bounded network connect behavior by default while daemon call sites can continue applying shorter configured peer timeouts. | Client SDKs + Network Runtime | `cargo test -p slskr-client`; resolver/raw-stream inventory classification. | Verified |
docs/dev/bug-burndown-ledger.md:77:| BUG-039 | Docs/Config | WebSocket auth examples | Medium | High | `docs/http-api-features.md` showed raw browser `new WebSocket(...)` examples without the required token subprotocol and a Node.js example using a constructor object/header pattern that does not match browser WebSocket usage. Examples now use the supported `slskr.api-token.<encoded-token>` subprotocol array. | Operators copying raw WebSocket examples get the same auth behavior enforced by the browser/SDK WebSocket gates. | Docs/Config + Frontend/API Handling | `scripts/check-websocket-auth-coverage.sh`; example Web API inventory classification; remediation baseline. | Verified |
docs/dev/bug-burndown-ledger.md:78:| BUG-040 | Network Runtime | Protocol count loop bounds | Medium | High | The first calibrated Rust protocol taint lens found unbounded wire-derived counts driving parser loops in peer search results, server string vectors/possible parents, and shared-file browse payload parsers. Counts now route through `Reader::read_bounded_count`, which rejects impossible counts based on remaining bytes and minimum bytes per item before loop execution. | Hostile count metadata can no longer force excessive parser iteration in these protocol and browse-payload paths. | Network Runtime + Backend/API | `scripts/check-rust-protocol-taint-lens.sh`; peer/server count regression tests; shared-file payload count regression tests; remediation baseline. | Verified |
docs/dev/bug-burndown-ledger.md:87:| `scripts/check-browser-token-persistence.sh` | Prevent browser API tokens and ListenBrainz tokens from returning to persistent `localStorage`. |
docs/dev/bug-burndown-ledger.md:90:| `scripts/check-csp-policy.sh` | Prevent broad inline CSP allowances and keep the WASM execution exception scoped to the Rust WASM shell. |
docs/dev/bug-burndown-ledger.md:92:| `scripts/check-rate-limit-proxy-policy.sh` | Ensure forwarded client IP rate-limit keys are used only behind explicitly trusted proxy CIDRs. |
docs/dev/bug-burndown-ledger.md:94:| `scripts/check-transfer-event-growth.sh` | Keep transfer event logs bounded by a byte-cap rotation path with regression coverage. |
docs/dev/bug-burndown-ledger.md:99:| `scripts/check-secret-scanning.sh` | Scan tracked files for committed secret patterns and verify local secret files remain ignored. |
docs/dev/bug-burndown-ledger.md:113:| `scripts/check-shell-script-hygiene.sh` | Run shell syntax checks and flag common script footguns. |
docs/dev/bug-burndown-ledger.md:116:| `scripts/check-remediation-script-registry.sh` | Ensure every `scripts/check-*.sh` gate is executable and registered in the baseline. |
docs/rust-ui-parity-ledger.md:16:running Soulseek session, real peers, real transfer state, Solid auth, or real
docs/rust-ui-parity-ledger.md:41:  delete, deny, restart, shutdown, and vacuum flows stay inside the Rust shell
docs/rust-ui-parity-ledger.md:42:- native live-response parsing now handles dotted and indexed payload paths,
docs/rust-ui-parity-ledger.md:52:- Browse now exposes tabbed peer sessions, cached folder state, breadcrumb
docs/rust-ui-parity-ledger.md:54:  download manifest so the page behaves closer to the expected session browser
docs/rust-ui-parity-ledger.md:60:  edit, note, audience, grant, token, inbound-access, and settings workflows
docs/rust-ui-parity-ledger.md:62:- Native rows now expose structured route data attributes for filenames, peers,
docs/rust-ui-parity-ledger.md:64:  paths, transfer states, and system areas. The WASM action resolver uses those
docs/rust-ui-parity-ledger.md:67:- Native action path resolution now separates selected route targets from
docs/rust-ui-parity-ledger.md:69:  share-group member edits, and share-grant update/token/backfill/delete actions
docs/rust-ui-parity-ledger.md:76:  queue, progress, permissions, owner, path, and next action after selection.
docs/rust-ui-parity-ledger.md:83:  share groups expose grant/token controls, and browse rows distinguish folder
docs/rust-ui-parity-ledger.md:93:  session/storage setup, collection share drafts, share-group member/token
docs/rust-ui-parity-ledger.md:94:  mutations, inbound share manifests, cached browse sessions, and operator tabs.
docs/rust-ui-parity-ledger.md:102:| Playlist Intake | Paste/upload shell, parsed row table, row correction controls, import validation, provider/MusicBrainz/SongID supporting tabs, and acquisition plan queue controls. | Validate file upload and provider enrichment against live provider fixtures. |
docs/rust-ui-parity-ledger.md:109:| Solid | Solid-specific identity/status shell, WebID resolve, session/connect controls, storage root state, linked-data sync controls, and related integration detail. | Validate auth/session transitions against a real Solid provider. |
docs/rust-ui-parity-ledger.md:111:| Share Groups | Group list, selected group detail, member picker, add/remove member controls, token issue/revoke, grant mutation controls, permission matrix, and selected grant/group payloads. | Validate revoke/update/member removal against live share-group records. |
docs/rust-ui-parity-ledger.md:112:| Shared With Me | Inbound grants/tokens table, manifest preview, owner/contact context, open/stream/backfill/copy token/leave controls, permission state, and selected grant payloads. | Validate exact token copy and stream/open behavior against live inbound grants. |
docs/rust-ui-parity-ledger.md:113:| Browse | Peer browser, tabbed sessions, cached tree state, breadcrumb persistence controls, folder expansion, file/folder split, file filter, multi-select manifest, and queue selected action payloads. | Validate cache restore against live browse status and folder payloads. |
scripts/run-slskdn-cross-client-interop.sh:16:    # shellcheck disable=SC1090
scripts/run-slskdn-cross-client-interop.sh:22:api_token="${SLSKR_CROSS_CLIENT_API_TOKEN:-slskr-cross-client-interop}"
scripts/run-slskdn-cross-client-interop.sh:101:  curl -fsS -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" "$url"
scripts/run-slskdn-cross-client-interop.sh:107:  curl -fsS -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" -H "Content-Type: application/json" -d "$payload" "$url"
scripts/run-slskdn-cross-client-interop.sh:112:  curl -fsS -X PUT -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" "$url"
scripts/run-slskdn-cross-client-interop.sh:189:account_password() {
scripts/run-slskdn-cross-client-interop.sh:208:slskr_password="${SLSKR_CROSS_CLIENT_SLSKR_PASSWORD:-$(account_password "$slskr_index")}"
scripts/run-slskdn-cross-client-interop.sh:210:slskdn_password="${SLSKR_CROSS_CLIENT_SLSKDN_PASSWORD:-$(account_password "$slskdn_index")}"
scripts/run-slskdn-cross-client-interop.sh:212:upstream_password="${SLSKR_CROSS_CLIENT_UPSTREAM_PASSWORD:-$(account_password "$upstream_index")}"
scripts/run-slskdn-cross-client-interop.sh:214:if [[ -z "$slskr_username" || -z "$slskr_password" || -z "$slskdn_username" || -z "$slskdn_password" ]]; then
scripts/run-slskdn-cross-client-interop.sh:307:slskr_remote_filename="$(basename "$slskr_share")/$slskr_fixture_name"
scripts/run-slskdn-cross-client-interop.sh:308:slskdn_remote_filename="shares\\\\$slskdn_fixture_name"
scripts/run-slskdn-cross-client-interop.sh:336:    password: admin
scripts/run-slskdn-cross-client-interop.sh:354:  password: "$slskdn_password"
scripts/run-slskdn-cross-client-interop.sh:366:  export SLSK_PASSWORD="$slskr_password"
scripts/run-slskdn-cross-client-interop.sh:370:  export SLSKR_API_TOKEN="$api_token"
scripts/run-slskdn-cross-client-interop.sh:378:  exec cargo run -q -p slskr -- serve
scripts/run-slskdn-cross-client-interop.sh:385:  exec "$slskdn_binary" --config "$slskdn_app/config/slskd.yml" --app-dir "$slskdn_app"
scripts/run-slskdn-cross-client-interop.sh:401:  local session
scripts/run-slskdn-cross-client-interop.sh:403:    if session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" 2>/dev/null)"; then
scripts/run-slskdn-cross-client-interop.sh:404:      if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:420:    if app="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null)"; then
scripts/run-slskdn-cross-client-interop.sh:437:  printf '\n[session]\n'
scripts/run-slskdn-cross-client-interop.sh:438:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:440:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:442:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" || true
scripts/run-slskdn-cross-client-interop.sh:444:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/run-slskdn-cross-client-interop.sh:447:try_request slskr-share-rescan auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" '{}' >/dev/null || true
scripts/run-slskdn-cross-client-interop.sh:448:try_request slskdn-share-rescan auth_put_empty "http://127.0.0.1:$slskdn_http_port/api/v0/shares" >/dev/null \
scripts/run-slskdn-cross-client-interop.sh:449:  || try_request slskdn-share-rescan-post auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/shares" '{}' >/dev/null \
scripts/run-slskdn-cross-client-interop.sh:454:  local path="$1"
scripts/run-slskdn-cross-client-interop.sh:458:    if [[ -f "$path" ]]; then
scripts/run-slskdn-cross-client-interop.sh:460:      actual_sha="$(sha256sum "$path" | awk '{print $1}')"
scripts/run-slskdn-cross-client-interop.sh:473:  if [[ -z "$upstream_username" || -z "$upstream_password" ]]; then
scripts/run-slskdn-cross-client-interop.sh:480:    SLSK_PASSWORD="$upstream_password" \
scripts/run-slskdn-cross-client-interop.sh:495:  local session listeners app endpoint escaped_slskr escaped_slskdn
scripts/run-slskdn-cross-client-interop.sh:496:  session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session")"
scripts/run-slskdn-cross-client-interop.sh:497:  if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-slskdn-cross-client-interop.sh:498:    record_check runtime-slskr-session ok "state=connected"
scripts/run-slskdn-cross-client-interop.sh:500:    record_check runtime-slskr-session fail "$session"
scripts/run-slskdn-cross-client-interop.sh:504:  listeners="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners")"
scripts/run-slskdn-cross-client-interop.sh:512:  app="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application")"
scripts/run-slskdn-cross-client-interop.sh:514:    record_check runtime-slskdn-session ok "server.isLoggedIn=true"
scripts/run-slskdn-cross-client-interop.sh:516:    record_check runtime-slskdn-session fail "$app"
scripts/run-slskdn-cross-client-interop.sh:528:  endpoint="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/endpoint")"
scripts/run-slskdn-cross-client-interop.sh:536:  endpoint="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/endpoint")"
scripts/run-slskdn-cross-client-interop.sh:553:  body="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse")"
scripts/run-slskdn-cross-client-interop.sh:564:  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/request" '{}' >/dev/null
scripts/run-slskdn-cross-client-interop.sh:565:  wait_json_contains protocol-slskr-browses-slskdn "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse" "$slskdn_fixture_name"
scripts/run-slskdn-cross-client-interop.sh:574:    SLSK_PASSWORD="${upstream_password:-$slskr_password}" \
scripts/run-slskdn-cross-client-interop.sh:593:    SLSK_PASSWORD="${upstream_password:-$slskdn_password}" \
scripts/run-slskdn-cross-client-interop.sh:611:  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:612:  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse/status" >>"$diag_file" 2>&1 || true
scripts/run-slskdn-cross-client-interop.sh:622:  if auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/messages" "{\"username\":\"$slskdn_username\",\"body\":\"$slskr_message\"}" >/dev/null; then
scripts/run-slskdn-cross-client-interop.sh:623:    wait_json_contains protocol-slskr-message-dispatch "http://127.0.0.1:$slskdn_http_port/api/v0/conversations/$escaped_slskr" "$slskr_message" || return 1
scripts/run-slskdn-cross-client-interop.sh:629:  if auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/conversations/$escaped_slskr" "\"$slskdn_message\"" >/dev/null; then
scripts/run-slskdn-cross-client-interop.sh:630:    wait_json_contains protocol-slskdn-message-dispatch "http://127.0.0.1:$slskr_http_port/api/v0/messages/$escaped_slskdn" "$slskdn_message" || return 1
scripts/run-slskdn-cross-client-interop.sh:641:  health="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/health")"
scripts/run-slskdn-cross-client-interop.sh:644:  stats="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/stats")"
scripts/run-slskdn-cross-client-interop.sh:647:  transport="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/transport")"
scripts/run-slskdn-cross-client-interop.sh:650:  ticket="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"peerId\":\"$slskr_username\",\"filename\":\"Interop/Test.flac\",\"expectedSize\":0}")"
scripts/run-slskdn-cross-client-interop.sh:658:  ticket="$(auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"filename\":\"Interop/Test.flac\",\"peerId\":\"$slskdn_username\"}")"
scripts/run-slskdn-cross-client-interop.sh:671:  local created transfer_id status bytes transfer_json download_path
scripts/run-slskdn-cross-client-interop.sh:673:      "http://127.0.0.1:$slskr_http_port/api/v0/transfers" \
scripts/run-slskdn-cross-client-interop.sh:674:      "{\"peer_username\":\"$slskdn_username\",\"filename\":\"$slskdn_remote_filename\",\"size\":$slskdn_fixture_size}" 2>&1)"; then
scripts/run-slskdn-cross-client-interop.sh:679:  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/transfers/$transfer_id/start" '{}' >/dev/null
scripts/run-slskdn-cross-client-interop.sh:682:    transfer_json="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/transfers/$transfer_id")"
scripts/run-slskdn-cross-client-interop.sh:686:      download_path="$slskr_state/downloads/shares\\$slskdn_fixture_name"
scripts/run-slskdn-cross-client-interop.sh:687:      wait_for_file "$download_path" "$slskdn_fixture_sha"
scripts/run-slskdn-cross-client-interop.sh:702:  local escaped_user response download_path
scripts/run-slskdn-cross-client-interop.sh:705:    "http://127.0.0.1:$slskdn_http_port/api/v0/transfers/downloads/$escaped_user" \
scripts/run-slskdn-cross-client-interop.sh:706:    "[{\"filename\":\"$slskr_remote_filename\",\"size\":$slskr_fixture_size}]")"
scripts/run-slskdn-cross-client-interop.sh:707:  download_path="$slskdn_app/downloads/$slskr_remote_filename"
scripts/run-slskdn-cross-client-interop.sh:708:  if wait_for_file "$download_path" "$slskr_fixture_sha"; then
scripts/run-slskdn-cross-client-interop.sh:712:  printf '%s\tslskr-to-slskdn-download\tfail\tdownload missing path=%s response=%s\n' "$(date -Is)" "$download_path" "$response" | tee -a "$result_file"
scripts/run-slskdn-cross-client-interop.sh:718:    printf '\n[final-session]\n'
scripts/run-slskdn-cross-client-interop.sh:719:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
scripts/run-slskdn-cross-client-interop.sh:721:    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
scripts/run-slskdn-cross-client-interop.sh:723:    auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
scripts/run-slskdn-cross-client-interop.sh:739:  slskr_session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session")"
scripts/run-slskdn-cross-client-interop.sh:740:  slskdn_app_json="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application")"
scripts/run-slskdn-cross-client-interop.sh:741:  [[ "$(printf '%s' "$slskr_session" | json_get state)" == "connected" ]]
docs/dev/bug-council-roslyn-analyzers.md:23:| CSL0004 | TaintToFilePath | High | Network-derived file/directory path without sanctioned containment validation. This catches hostile paths before filesystem sinks trust them. |
scripts/collect-upstream-parity-delta.sh:11:  local path="$2"
scripts/collect-upstream-parity-delta.sh:13:  if [[ ! -d "$path/.git" ]]; then
scripts/collect-upstream-parity-delta.sh:14:    printf '## %s\n\nMissing repository: %s\n\n' "$label" "$path"
scripts/collect-upstream-parity-delta.sh:19:  printf 'Repository: `%s`\n\n' "$path"
scripts/collect-upstream-parity-delta.sh:23:  git -C "$path" log --since="$since" --date=short --format='%H%x09%s' |
scripts/collect-upstream-parity-delta.sh:31:        *security*|*Harden*|*harden*|*bind*|*auth*|*CSRF*|*token*|*validation*)
docs/release.md:3:This is the release-prep path for binary archives. `slskr` is a single Rust
docs/release.md:12:This runs public-posture checks, shell syntax checks, Rust formatting, clippy,
docs/release.md:15:audit, and subpath smoke checks.
docs/release.md:25:executes the Rust web UI headless parity audit and the hermetic local
docs/release.md:28:credentialed public-live job: when the `SLSKR_LIVE_INTEROP_ENV` repository secret
docs/release.md:31:`target/live-interop`; when the secret is absent, it uploads an explicit skipped
docs/http-api-deployment.md:7:`slskr serve` binds to `127.0.0.1:5030` by default. Loopback-only binds may run without API auth when no token is configured.
docs/http-api-deployment.md:11:curl http://127.0.0.1:5030/api/health
docs/http-api-deployment.md:27:- `Authorization: Bearer <token>`
docs/http-api-deployment.md:28:- `X-API-Key: <token>`
docs/http-api-deployment.md:29:- `slskr.session` cookie only when `SLSKR_API_COOKIE_AUTH_ENABLED=true`
docs/http-api-deployment.md:31:Health/version/capability bootstrap routes remain public: `/`, `/api/health`, `/api/version`, `/api/session/enabled`, and `/api/v0/capabilities`.
docs/http-api-deployment.md:35:Terminate TLS at the proxy and forward to the loopback-bound daemon where possible.
docs/http-api-deployment.md:46:        proxy_pass http://127.0.0.1:5030;
docs/http-api-deployment.md:47:        proxy_http_version 1.1;
docs/http-api-deployment.md:48:        proxy_set_header Host $host;
docs/http-api-deployment.md:49:        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
docs/http-api-deployment.md:50:        proxy_set_header Forwarded "for=$remote_addr;proto=$scheme;host=$host";
docs/http-api-deployment.md:51:        proxy_set_header Upgrade $http_upgrade;
docs/http-api-deployment.md:52:        proxy_set_header Connection $connection_upgrade;
docs/http-api-deployment.md:57:Set `SLSKR_TRUSTED_PROXY_CIDRS` or `[auth].trusted_proxy_cidrs` to the proxy source CIDRs before relying on `Forwarded` or `X-Forwarded-For` for anonymous rate-limit keys. slskr ignores forwarded client IP headers from untrusted peers so direct clients cannot spoof another address.
docs/http-api-deployment.md:65:Before applying, create real secrets from placeholders in `k8s/secrets.example.yaml`:
docs/http-api-deployment.md:69:kubectl -n slskr create secret generic slskr-secrets \
docs/http-api-deployment.md:72:  --from-literal=SLSK_PASSWORD="<password>"
docs/http-api-deployment.md:84:Prometheus-compatible metrics are served at both `/api/metrics` and `/api/v0/metrics`. These routes are protected when API auth is enabled. The Kubernetes ServiceMonitor uses `/api/metrics` on the `http` service port and reads `SLSKR_API_TOKEN` from `slskr-secrets` as a bearer token.
docs/webui-endpoints.txt:84:GET /session
docs/webui-endpoints.txt:85:GET /session/enabled
docs/webui-endpoints.txt:201:POST /session
docs/webui-endpoints.txt:204:POST /share-grants/:var/token
docs/dev/bug-council-scan-registry.md:39:| Untrusted-string-to-path | Find file-system operations on caller-supplied strings without containment. |
docs/dev/bug-council-scan-registry.md:40:| Security-sensitive material | Find high-confidence private keys and token patterns. |
docs/dev/bug-council-scan-registry.md:41:| Red-team abuse lens | Re-check accepted fixes from an attacker viewpoint: spoofed identity, secret disclosure, confused deputy, replay, SSRF/path/process escape, and operational downgrade. |
scripts/check-bug-council-all-phases.sh:42:  printf 'Council all-phases runner is missing or not executable: %s\n' "${runner#"$repo_root"/}" >&2
scripts/check-compatibility-noop-documentation.sh:21:for route in '/api/options' '/api/options/yaml' '/api/options/yaml/location' 'logs/cache/bridge/config/bans/share-grant token' 'share-grant token/backfill' 'MusicBrainz subscription'; do
scripts/check-compatibility-noop-documentation.sh:23:    printf 'compatibility no-op documentation check failed: expected inventory token missing: %s\n' "$route" >&2
scripts/check-compatibility-noop-documentation.sh:30:    printf 'compatibility no-op documentation check failed: expected implementation/docs token missing: %s\n' "$expected" >&2
scripts/check-client-sdk-gates.sh:50:    find client-python -type d \( -name __pycache__ -o -name '*.egg-info' \) -prune -exec rm -rf {} +
scripts/check-client-sdk-gates.sh:65:client = SlskrClient("http://localhost:8080", "test-token")
scripts/check-client-sdk-gates.sh:66:assert client.base_url == "http://localhost:8080"
docs/dev/bug-council-active-backlog.md:24:| `Protocol count/length candidates` | 46 | Guarded | Current count/length candidates are classified; accepted raw-frame, transfer-chunk, and protocol loop-bound bugs are fixed, with taint/adversarial gates covering high-risk parser paths. | Reopen only when a fresh wire-derived allocation/read/loop flow is not covered by BUG-033, BUG-035, BUG-040, or existing bounded-count evidence. |
docs/dev/bug-council-active-backlog.md:25:| `Protocol scalar emission candidates` | 42 | Guarded | Current scalar emission candidates are classified; accepted API-to-protocol narrowing bugs are fixed and protocol-visible length/code emissions are checked or inventory-tested. | Reopen only when a fresh protocol-visible narrowing path lacks a checked conversion or discriminant inventory evidence. |
docs/dev/bug-council-active-backlog.md:26:| `Resolver/raw stream candidates` | 225 | Guarded | Current raw stream candidates are classified; accepted raw-frame and connect-timeout bugs are fixed, and daemon/SDK stream paths are covered by timeout, resolver, and frame-size guards. | Reopen only when a fresh direct socket/resolver/read path lacks timeout, address policy, or size-bound evidence. |
docs/dev/bug-council-active-backlog.md:27:| `Task/cancellation/lifecycle candidates` | 246 | Guarded | Current lifecycle candidates are classified; accepted TypeScript timer/default bugs are fixed, and daemon/WebSocket/webhook task ownership is covered by bounded channels, timeouts, and shutdown tests. | Reopen only when a fresh spawn/timeout/channel path lacks shutdown, bounded queue, or cleanup evidence. |
docs/dev/bug-council-active-backlog.md:28:| `Example Web API candidates` | 287 | Guarded | Current web/API examples are classified; stale WebSocket auth examples are fixed and remaining examples are covered by token, unsafe-open, CORS, and docs freshness gates. | Reopen only when a fresh example bypasses the SDK auth helpers, hard-codes secrets, or contradicts deployment posture. |
scripts/check-audit-tooling.sh:20:    printf 'audit tooling check failed: expected audit token missing: %s\n' "$expected" >&2
docs/http-api-features.md:89:Batch endpoints allow you to execute multiple API operations in a single HTTP request, reducing round-trip time and overhead.
docs/http-api-features.md:102:      "path": "/api/stats"
docs/http-api-features.md:107:      "path": "/api/transfers"
docs/http-api-features.md:112:      "path": "/api/searches",
docs/http-api-features.md:160:    {"id":"s","method":"GET","path":"/api/stats"},
docs/http-api-features.md:161:    {"id":"t","method":"GET","path":"/api/transfers"},
docs/http-api-features.md:162:    {"id":"m","method":"GET","path":"/api/messages"}
docs/http-api-features.md:172:    {"id":"1","method":"POST","path":"/api/messages","body":"{\"recipient\":\"alice\",\"content\":\"Hi\"}"},
docs/http-api-features.md:173:    {"id":"2","method":"POST","path":"/api/messages","body":"{\"recipient\":\"bob\",\"content\":\"Hello\"}"},
docs/http-api-features.md:174:    {"id":"3","method":"POST","path":"/api/messages","body":"{\"recipient\":\"charlie\",\"content\":\"Hey\"}"}
docs/http-api-features.md:184:    {"id":"1","method":"DELETE","path":"/api/transfers/123"},
docs/http-api-features.md:185:    {"id":"2","method":"DELETE","path":"/api/transfers/124"},
docs/http-api-features.md:186:    {"id":"3","method":"DELETE","path":"/api/transfers/125"}
docs/http-api-features.md:196:- **Timing**: See total execution time in response
docs/http-api-features.md:205:  headers: {'Authorization': 'Bearer token'},
docs/http-api-features.md:272:const token = sessionStorage.getItem('slskr-token') ?? '';
docs/http-api-features.md:273:const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
docs/http-api-features.md:295:const token = process.env.SLSKR_API_TOKEN || '';
docs/http-api-features.md:296:const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
docs/http-api-features.md:313:    console.log(`Transfer started: ${event.data.filename}`);
docs/http-api-features.md:315:    console.log(`Transfer completed: ${event.data.filename}`);
docs/http-api-features.md:359:curl -H "Authorization: Bearer token" \
docs/http-api-features.md:360:     http://localhost:8080/api/cache/stats
docs/http-api-features.md:403:curl -X POST -H "Authorization: Bearer token" \
docs/http-api-features.md:404:     http://localhost:8080/api/cache/invalidate \
docs/http-api-features.md:421:curl http://localhost:8080/api/health
docs/http-api-features.md:424:curl http://localhost:8080/api/version
docs/http-api-features.md:427:curl http://localhost:8080/api/capabilities
docs/http-api-features.md:430:curl -H "Authorization: Bearer token" \
docs/http-api-features.md:431:     http://localhost:8080/api/stats
docs/http-api-features.md:437:curl -H "Authorization: Bearer token" \
docs/http-api-features.md:438:     http://localhost:8080/api/metrics
docs/http-api-features.md:540:  path: `/api/transfers/${t.id}`
docs/http-api-features.md:555:const token = sessionStorage.getItem('slskr-token') ?? '';
docs/http-api-features.md:556:const protocols = token ? [`slskr.api-token.${encodeURIComponent(token)}`] : [];
docs/http-api-features.md:569:curl -H "Authorization: Bearer token" \
docs/http-api-features.md:570:     http://localhost:8080/api/cache/stats | jq '.hit_rate'
docs/http-api-features.md:592:1. Check if server supports WebSocket (may be behind reverse proxy)
docs/http-api-features.md:594:3. Check firewall/proxy settings for WebSocket support
docs/http-api-features.md:602:2. Increase server timeout in reverse proxy
docs/http-api-features.md:619:- [RFC 6455 - WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
docs/http-api-features.md:620:- [JSON Batch Request/Response RFC](https://jsonapi.org/ext/jsonpatch/)
docs/live-interop-test-matrix.md:13:| Client | Role in matrix | Local path | Current status |
docs/live-interop-test-matrix.md:25:| Login/session | New/existing account login succeeds, server greeting/hash parsed, relogin behavior handled | `login smoke`, live matrix | Unit suites plus daemon login/address probes |
docs/live-interop-test-matrix.md:30:| Indirect connection | `ConnectToPeer` / `PierceFirewall` works for firewalled/NAT paths | `probe indirect-peer`, local peer smoke | `slskr` self-smoke proves protocol; daemon payload coverage now uses queued direct/NAT-PMP transfer probes |
docs/live-interop-test-matrix.md:31:| Distributed path | Distributed `D` peer init accepts ping/probe traffic | `probe distributed-peer` | Runtime/library unit coverage; daemon live probe still optional |
docs/live-interop-test-matrix.md:32:| File-transfer init | Transfer `F` peer init is accepted only as part of a real queued transfer | `probe file-transfer-peer` against `slskr` self-smoke | Raw token-echo closes with EOF against `slskr`/`slskr` as expected; queued payload transfer proof now covers daemon bytes |
docs/live-interop-test-matrix.md:35:| Download/upload bytes | Queued download opens transfer path, moves bytes, reports completion | `probe download-peer` added for negotiated file payload reads | Daemon payload proof runs after browse exposes exact fixture path and target remains connected |
docs/live-interop-test-matrix.md:50:- `.github/workflows/live-parity.yml`: scheduled/manual CI proof for the Rust web UI headless parity audit plus hermetic local `slskd_api` automation-client smoke, with screenshots, web assets, and daemon logs uploaded as artifacts. It also runs the credentialed public live matrix when the `SLSKR_LIVE_INTEROP_ENV` secret is configured, or uploads a skipped TSV when live credentials are intentionally absent.
docs/live-interop-test-matrix.md:62:Use these when public Soulseek resets or throttles daemon sessions:
docs/live-interop-test-matrix.md:92:For account rotation, add more `SLSKR_TEST_N_USERNAME` / `SLSKR_TEST_N_PASSWORD` pairs to `.env`, then point the index variables above at non-colliding accounts. For VPN rotation, keep each daemon/probe account pinned to one stable egress for the duration of a run; do not change egress mid-session.
docs/live-interop-test-matrix.md:94:## Pairwise execution plan
docs/live-interop-test-matrix.md:101:| 4. Search/browse | Search for fixture token, browse target shares, verify expected fixture path/metadata |
docs/live-interop-test-matrix.md:106:| 9. Negative | Offline target, blocked transfer, bad room, bad peer token, closed listener, auth failure |
docs/live-interop-test-matrix.md:121:| browser/player E2E against daemon state | Passed | 2026-05-04 Playwright `e2e/live-surfaces.spec.ts`: this repo's web bundle was hosted by adjacent `slskr`; search, downloads, uploads, messages, rooms, browse, system, and player shell controls loaded without runtime errors |
docs/live-interop-test-matrix.md:134:| daemon raw transfer-token probe | Diagnostic-only non-blocking row | `slskr` and `slskr` close the raw token echo probe with EOF because no transfer is queued; real payload proof is now covered by queued requester-listener probes |
docs/live-interop-test-matrix.md:136:| `slskr` live login after repeated daemon retries | Mitigated by VPN account pool | Fresh p5-p8 accounts avoid the prior host-egress reset path for focused daemon/probe runs; raw public-host reruns may still reset under heavy retry |
docs/live-interop-test-matrix.md:138:| `slskr` live `slskd_api` automation client smoke | Passed | 2026-05-05 local daemon run with Python `slskd_api` 0.2.4: `SLSKR_SLSKD_API_SMOKE_TOKEN=slskd-api-smoke-token scripts/run-slskd-api-compat-smoke.sh` passed 91 client calls across application, session, server, search, transfer, room, conversation, user, file, relay, share, options, events, logs, and telemetry APIs |
docs/live-interop-test-matrix.md:143:## Bugs fixed during matrix execution
docs/live-interop-test-matrix.md:150:| Runtime unit tests | `SearchInternal` mismatched-token test expected an exception even though current behavior safely ignores stale token responses. | Updated the test expectation to assert no exception and no callback. |
docs/live-interop-test-matrix.md:155:| Browse/download coverage | The matrix only had synthetic peer and raw transfer-token probes. | Added `browse-peer` and `download-peer`; daemon runner now attempts browse fixture proof and negotiated fixture download before the legacy raw token probe. |
docs/live-interop-test-matrix.md:161:| Browser/player E2E gap | Browser/player coverage stopped at unit/build checks and did not mount this repo's bundle against daemon state. | Added a Playwright live-surface spec and made the slskr E2E harness portable: it can build this repo's `web` bundle, host it from adjacent `slskr`, and assert route/player shell behavior. |
docs/live-interop-test-matrix.md:163:| Restart log diagnostics | slskr restart logs contained null padding and HTTP readiness relied on logs because the daemon bound HTTP to loopback inside the namespace. | TSV sanitization now strips null bytes, and daemon launches set `ASPNETCORE_URLS=http://0.0.0.0:$port` for reachable HTTP readiness where the target honors ASP.NET Core URL binding. |
docs/live-interop-test-matrix.md:167:| Probe port metadata lag | Back-to-back payload probes could advertise different forwarded ports faster than public-server peer metadata converged. | Daemon download probes reuse the same local NAT-PMP port per target run so the forwarded public port remains stable for text and binary fixture transfers. |
docs/live-interop-test-matrix.md:168:| `slskr` obfuscated response framing | Diagnostic mode proved `slskr` accepts obfuscated init/request but sends `UserInfoResponse` as a plain frame on that connection. | `obfuscated-peer` now falls back to a plain peer-message response after the primary obfuscated-response read times out or EOFs, preserving compatibility without weakening the initial obfuscated request path. |
docs/app-surface.md:7:- `slskr serve`: run the bundled app shell and daemon scaffold. Defaults to `SLSKR_HTTP_BIND=127.0.0.1:5030`.
docs/app-surface.md:11:- `slskr smoke local-peer`: two-account local peer path smoke.
docs/app-surface.md:27:- one Soulseek session lifecycle
docs/app-surface.md:36:The daemon calls `slskr-client` for protocol/runtime behavior rather than duplicating connection logic in the app crate. The current scaffold can optionally start a real server login session when credentials are provided through the environment.
docs/app-surface.md:45:- one pod-friendly daemon process that owns the Soulseek session, HTTP API, WebUI, event stream, static assets, share scanner, transfer engine, runtime telemetry, and integration callbacks
docs/app-surface.md:61:enabled so short-lived authorization flows survive daemon restarts until their
docs/app-surface.md:67:token/backfill helpers, profile updates, and MusicBrainz release-radar
docs/app-surface.md:76:The first app shell exposes:
docs/app-surface.md:78:- `GET /`: bundled local dashboard for session, listener, share, search, browse, message, room, user, catalog, and transfer projections, with controls for session connect/ping/disconnect/privilege-check, starting/completing searches, watching/unwatching users, requesting browse, rescanning shares, queueing/updating transfer projections with explicit progress bytes, sending/acknowledging private messages, joining/leaving rooms, syncing the server room list, sending room messages with an explicit sender, filtering search/transfer/catalog/message/room/browse tables, refreshing projection tables, and using the configured API token as an in-memory browser bearer token for protected APIs
docs/app-surface.md:87:- `GET /api/v0/stats`: compact aggregate counts for session, listeners, shares, searches, users, browse cache, messages, rooms, transfers, and durable database/projection health
docs/app-surface.md:88:- `GET /api/v0/metrics`: Prometheus-style text counters/gauges for session, listeners, shares, searches, users, browse cache, messages, rooms, transfers, runtime operations, and persisted SQLite row counts
docs/app-surface.md:89:- `GET /api/v0/telemetry`: protected JSON runtime health snapshot with sanitized config flags, listener/session state, storage status, database health, and projection counts
docs/app-surface.md:90:- `GET /api/v0/events`: bounded event log for recent search, transfer, share, user, browse, message, room, listener, relay, bridge, mesh, security, library, integration, player, telemetry, and session workflows. Supports `kind`, `topic`, `q`, `limit`, and `offset` query parameters. When persistence is enabled, event rows hydrate from and write through to SQLite.
docs/app-surface.md:94:- `GET /api/v0/shares/catalog`: deterministic sorted share catalog with virtual path, size, extension, attribute count, file count, filtered count, and total bytes. Supports `q`, `prefix`, `extension`, `limit`, and `offset` query parameters.
docs/app-surface.md:95:- `GET /api/v0/files/:root`: list files and immediate directory summaries under one configured share-root label without exposing local host paths. Supports `q`, `folder`/`path`/`prefix`, `recursive`, `extension`, `limit`, and `offset` query parameters; the default flat root listing is preserved for compatibility.
docs/app-surface.md:100:- `POST /api/v0/server` and `DELETE /api/v0/server`: aliases for session connect/disconnect.
docs/app-surface.md:101:- `GET /api/v0/session/enabled`: reports whether HTTP auth is enabled.
docs/app-surface.md:103:- `GET /api/v0/searches/records`: list search records with the slskr metadata envelope (`entries`, counts, pagination, and next token) for the dashboard and richer native callers.
docs/app-surface.md:105:- `GET /api/v0/searches/:token`: read one search record
docs/app-surface.md:106:- `POST /api/v0/searches/:token/complete`: mark one search record completed
docs/app-surface.md:108:- `POST /api/v0/search-responses`: merge one flattened result into a search record from JSON body with `token`, `filename`, `size`, and optional `peer_username`, `extension`, `slot_free`, `average_speed`, and `queue_length`
docs/app-surface.md:120:- `POST /api/v0/browse-responses`: merge flattened browse results from JSON body with `username` plus either one `filename`/`size`/optional `extension` or an `entries` array of those fields. Optional `complete:false` records the browse as `partial`; omitted or true promotes it to `ready`. Browse result projections write through to SQLite when persistence is enabled.
docs/app-surface.md:138:- `GET/POST/PUT/DELETE /api/share-grants` plus collection lookup/token/backfill helpers: maintain share-grant projections with SQLite write-through when persistence is enabled
docs/app-surface.md:140:- `GET /api/session`: current Soulseek session state, including last server message name and counters
docs/app-surface.md:141:- `GET /api/v0/session`: versioned alias for current session state
docs/app-surface.md:148:- `POST /api/v0/transfers`: create a queued transfer projection from JSON body with `filename` and optional `direction`, `peer_username`, `local_path`, and `size`. When persistence is enabled, create/start/retry/progress/complete/cancel/delete/prune and replacement mutations write through to SQLite, transition/progress events append to SQLite `transfer_events`, and the reloadable transfer projection remains mirrored in `transfer-state.json`.
docs/app-surface.md:150:- `POST /api/v0/transfers/:id/start`: mark a transfer in progress; when `local_path` is present without a peer, validate the file on disk and complete or fail the projection from real metadata; when `peer_username` is present, request peer-address metadata, negotiate a peer-message `TransferRequest`/`TransferResponse`, and use the direct `F` connection token/offset handshake for local-path upload/download streaming, preferring type-1 obfuscated `F` init when advertised and falling back to plain direct `F`; if direct `F` connect fails, request server-mediated `ConnectToPeer`/`PierceFirewall` and retry the same file stream over the indirect socket. Inbound peer transfer requests for locally indexed share files are accepted and served over incoming direct `F`, `PeerInit F`, or `PierceFirewall` file-transfer sockets.
docs/app-surface.md:158:- `POST /api/session/connect`: start a session using configured environment credentials
docs/app-surface.md:159:- `POST /api/v0/session/connect`: versioned alias for session connect
docs/app-surface.md:160:- `POST /api/session/disconnect`: drop the active session and suppress reconnect
docs/app-surface.md:161:- `POST /api/v0/session/disconnect`: versioned alias for session disconnect
docs/app-surface.md:162:- `POST /api/session/ping`: request an immediate server ping when connected
docs/app-surface.md:163:- `POST /api/v0/session/ping`: versioned alias for session ping
docs/app-surface.md:164:- `POST /api/session/privileges/check`: request a server privilege-time check when connected; matching responses update `session.privileges_seconds`
docs/app-surface.md:165:- `POST /api/v0/session/privileges/check`: versioned alias for session privilege check
docs/app-surface.md:177:- `SLSKR_AUTO_CONNECT` to control startup login behavior; defaults to true only when username and password are configured
docs/app-surface.md:178:- `SLSKR_RECONNECT` to reconnect after session I/O failure; defaults to the auto-connect value
docs/app-surface.md:187:- `SLSKR_SHARE_DIRS` for semicolon-separated share roots. The daemon scans explicit roots into virtual `root/file` paths at startup.
docs/app-surface.md:189:- `SLSKR_SHARE_INCLUDE_HIDDEN` to include dot-prefixed path components; defaults to `false`
docs/app-surface.md:192:- `SLSKR_SHARE_FIXTURE` for temporary in-memory test entries as `path=size;path=size`
docs/app-surface.md:197:- `SLSKR_API_TOKEN` for HTTP API auth. If set, protected API routes accept `Authorization: Bearer <token>` and automation-compatible `X-API-Key: <token>`. Legacy same-site `slskr.session` cookie auth is accepted only when `SLSKR_API_COOKIE_AUTH_ENABLED=true`. Browser clients should keep tokens in memory or session storage rather than long-lived persistent storage.
docs/app-surface.md:199:- `SLSKR_AUTH_DISABLED` to explicitly disable HTTP API auth. Loopback-only binds default to disabled when no token is configured; non-loopback binds require a token unless this is set.
docs/app-surface.md:200:- `SLSKR_PERSISTENCE_ENABLED` enables the default-off SQLite persistence path. Share index, event log, search, transfer, user, browse, message, room, collection, library, destination, now-playing, wishlist, contact, sharegroup, share-grant, user-note, interest, security-ban, OAuth-state, webhook, and runtime compatibility projections write through to SQLite; transfer projection state also remains restart-safe through `transfer-state.json`.
docs/app-surface.md:201:- Spotify integration uses the existing slskr HTTP/WebUI port for OAuth callback handling. Configure `SLSKR_SPOTIFY_ENABLED=true` and `SLSKR_SPOTIFY_CLIENT_ID`; if `SLSKR_SPOTIFY_REDIRECT_URI` is unset, the daemon advertises `http://127.0.0.1:<http-port>/api/integrations/spotify/callback` for loopback use. The callback requires a daemon-issued cryptographically random state value, expires pending state after 10 minutes, and rejects replayed, missing, or invalid state.
docs/app-surface.md:203:- `SLSKR_EXTERNAL_VISUALIZER_COMMAND` configures the optional local visualizer launch command. `SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED=true` is also required before the daemon will spawn that command; launch attempts are recorded as events.
docs/app-surface.md:204:- `SLSK_SERVER`, `SLSK_LISTEN_PORT`, `SLSK_USERNAME`, and `SLSK_PASSWORD` for the initial session scaffold
docs/app-surface.md:205:- gitignored `.secrets/` files for local lab credentials
docs/app-surface.md:206:- OpenBao paths documented under `../k3s/slskr/README.md`
docs/app-surface.md:212:- separate secret loading path for credentials
docs/app-surface.md:213:- maintained service/container artifacts that follow [install.md](./install.md) and never require checked-in secrets
docs/app-surface.md:221:- reverse-proxy guidance
docs/app-surface.md:223:Current behavior: bearer-token and `X-API-Key` auth are available for protected API routes; legacy same-site dashboard cookie auth is disabled unless explicitly opted in. Auth is required automatically for non-loopback HTTP binds unless `SLSKR_AUTH_DISABLED=true` is set. When auth is enabled, unsafe API methods reject cross-site browser requests with foreign `Origin` or `Referer` headers. `GET /`, `GET /api/health`, `GET /api/version`, `GET /api/session/enabled`, and `GET /api/v0/capabilities` remain public health/version/capability surfaces.
docs/app-surface.md:227:Spotify is the only current integration in this group with a true OAuth clickthrough. slskr serves `/api/integrations/spotify/callback` on the same HTTP port as the WebUI/API, so operators do not need to expose a second listener. The WebUI shows the exact redirect URI to register in the Spotify developer dashboard. Authorization requests use a server-side state store with cryptographically random state, a 10-minute expiry, single-use consumption before the callback is accepted, and SQLite hydration/write-through when persistence is enabled.
docs/app-surface.md:260:  without exposing local host paths
scripts/run-council-bughunt.sh:5:exec "$repo_root/scripts/run-bug-council-all-phases.sh"
scripts/build-release-archive.sh:17:  - slskr executable
scripts/build-release-archive.sh:111:binary_path="target/$target/$profile/$binary_name"
scripts/build-release-archive.sh:112:if [[ ! -f "$binary_path" ]]; then
scripts/build-release-archive.sh:113:  binary_path="target/$profile/$binary_name"
scripts/build-release-archive.sh:115:if [[ ! -f "$binary_path" ]]; then
scripts/build-release-archive.sh:126:cp "$binary_path" "$stage_dir/$binary_name"
scripts/build-release-archive.sh:140:SLSKR_CONFIG=/path/to/config.toml or environment variables. Start from
scripts/build-release-archive.sh:149:import pathlib
scripts/build-release-archive.sh:152:archive = pathlib.Path(os.environ["ARCHIVE"])
scripts/build-release-archive.sh:153:root = pathlib.Path(os.environ["DIST_DIR"]) / os.environ["ROOT_NAME"]
scripts/build-release-archive.sh:155:    for path in root.rglob("*"):
scripts/build-release-archive.sh:156:        if path.is_file():
scripts/build-release-archive.sh:157:            zf.write(path, path.relative_to(root.parent).as_posix())
scripts/check-rust-protocol-taint-lens.sh:61:    ! -path '*/target/*' \
scripts/check-rust-protocol-taint-lens.sh:62:    ! -path '*/tests/fixtures/*' \
docs/WEBHOOK_API.md:7:`POST /api/webhooks` and `POST /api/admin/webhooks` return the webhook signing secret only in the creation response. Treat this as a one-time display value. List, detail, delete, patch, test, and log routes do not return webhook secrets.
docs/WEBHOOK_API.md:9:If the creation response is lost, delete and recreate the webhook with a new generated secret or provide a new explicit `secret` field at creation time.
docs/WEBHOOK_API.md:22:Authorization: Bearer <api-token>
docs/WEBHOOK_API.md:25:{"url":"https://example.com/slskr/webhook","events":"search.created,transfer.completed"}
docs/WEBHOOK_API.md:33:  "secret": "secret_generated_value",
docs/WEBHOOK_API.md:34:  "secretReturnedOnce": true,
docs/WEBHOOK_API.md:43:Authorization: Bearer <api-token>
docs/WEBHOOK_API.md:46:List responses include id, URL, events, active state, retry settings, and timestamps. They intentionally omit `secret`.
scripts/run-live-interop-matrix.sh:6:extra_env_file="${SLSKR_LIVE_EXTRA_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
scripts/run-live-interop-matrix.sh:7:pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
scripts/run-live-interop-matrix.sh:68:  local path="${!var_name:-}"
scripts/run-live-interop-matrix.sh:69:  if [[ -z "$path" ]]; then
scripts/run-live-interop-matrix.sh:73:  if [[ "$path" != /* ]]; then
scripts/run-live-interop-matrix.sh:74:    path="$repo_root/$path"
scripts/run-live-interop-matrix.sh:76:  if [[ ! -f "$path" ]]; then
scripts/run-live-interop-matrix.sh:80:  printf '%s' "$path"
scripts/run-live-interop-matrix.sh:129:  local password="${!pass_var}"
scripts/run-live-interop-matrix.sh:136:  SLSK_PASSWORD="$password" \
docs/CLIENT_LIBRARIES.md:34:        base_url="http://localhost:8080",
docs/CLIENT_LIBRARIES.md:35:        token="your-api-key-here"
docs/CLIENT_LIBRARIES.md:81:await client.create_transfer(direction, peer_username, filename)
docs/CLIENT_LIBRARIES.md:89:# Build and execute batch operations
docs/CLIENT_LIBRARIES.md:96:response = await batch.execute()
docs/CLIENT_LIBRARIES.md:136:async with SlskrClient("http://localhost:8080", "token") as client:
docs/CLIENT_LIBRARIES.md:177:    token: str,                 # API authentication token
docs/CLIENT_LIBRARIES.md:199:- `create_transfer(direction, peer_username, filename)` → dict
docs/CLIENT_LIBRARIES.md:209:`slskr` with the daemon base URL and API token. The compatibility surface covers
docs/CLIENT_LIBRARIES.md:210:the slskd-style application, session, server, search, transfer, room,
docs/CLIENT_LIBRARIES.md:218:    host="http://127.0.0.1:5030",
docs/CLIENT_LIBRARIES.md:219:    api_key="your-api-token",
docs/CLIENT_LIBRARIES.md:229:SLSKR_SLSKD_API_SMOKE_TOKEN=slskd-api-smoke-token scripts/run-slskd-api-compat-smoke.sh
docs/CLIENT_LIBRARIES.md:234:- `SLSKR_SLSKD_API_SMOKE_TOKEN`: required bearer token for the temporary authenticated daemon.
docs/CLIENT_LIBRARIES.md:268:        "http://localhost:8080",
docs/CLIENT_LIBRARIES.md:269:        "your-api-key-here",
docs/CLIENT_LIBRARIES.md:450:NewClient(baseURL, token string) *Client
docs/CLIENT_LIBRARIES.md:468:builder.Get(path, opID)
docs/CLIENT_LIBRARIES.md:469:builder.Post(path, body, opID)
docs/CLIENT_LIBRARIES.md:470:builder.Put(path, body, opID)
docs/CLIENT_LIBRARIES.md:471:builder.Delete(path, opID)
docs/CLIENT_LIBRARIES.md:495:    baseURL: 'http://localhost:8080',
docs/CLIENT_LIBRARIES.md:496:    apiKey: 'your-api-key'
docs/CLIENT_LIBRARIES.md:619:        # 401 - Invalid token
docs/CLIENT_LIBRARIES.md:665:response = await batch.execute()
scripts/check-dev-tooling.sh:42:note_optional_command shellcheck "shell lint"
scripts/check-dev-tooling.sh:47:note_optional_command tmux "live soak sessions"
docs/dev/council-scan-inventory.md:41:| Task/cancellation/lifecycle candidates | Fixed | Re-run after new spawn, timeout, interval, channel, cancellation, or shutdown code is added; BUG-037 and BUG-038 cover accepted TypeScript lifecycle bugs. |
docs/dev/council-scan-inventory.md:93:| `decode_file_entries`, `decode_string_vec`, `decode_possible_parents`, and shared-file browse payload parsers | Protocol + Backend/API | Fixed | Medium | High | BUG-040: calibrated Rust protocol taint lens found reader-derived counts flowing into parser loops; counts now route through `Reader::read_bounded_count` and reject impossible counts based on remaining bytes before loop execution. | `scripts/check-rust-protocol-taint-lens.sh`; peer/server/shared-file count regression tests. |
docs/dev/council-scan-inventory.md:99:| JSON search response token to protocol `u32` | Backend/API | Fixed | Medium | High | BUG-034: `/api/search-responses` now rejects tokens above `u32::MAX` instead of narrowing with `as u32`. | `search_response_api_rejects_oversized_protocol_token` |
docs/dev/council-scan-inventory.md:128:| Daemon session manager task | Backend/API + Network Runtime | Existing Guard | Low | High | Session commands use bounded `mpsc`, receive/readiness are wrapped in timeouts, reconnect uses configured delay, and the task exits when the command channel closes. | Session and API route tests. |
docs/dev/council-scan-inventory.md:131:| Webhook delivery tasks | Backend/API + Release/Ops | Existing Guard | Low | High | Registered webhooks are capped, delivery concurrency is capped by semaphore, timeouts are clamped, redirects are disabled, and delivery pool saturation drops work instead of queueing unbounded requests. | Webhook tests and outbound policy gate. |
docs/dev/council-scan-inventory.md:133:| CLI/tests/examples sleeps, spawns, and contract timeouts | Tests/Tooling | False Positive | Low | High | Remaining lifecycle hits are bounded CLI diagnostics, local smoke tests, contract servers, examples, or README snippets. | Keep scoped to test/example review unless promoted by failing gates. |
docs/dev/council-scan-inventory.md:139:| `docs/http-api-features.md` raw browser WebSocket examples | Docs/Config + Frontend/API Handling | Fixed | Medium | High | BUG-039: raw browser examples now pass the `slskr.api-token.<encoded-token>` WebSocket subprotocol instead of omitting auth. | `scripts/check-websocket-auth-coverage.sh`; docs freshness/baseline. |
docs/dev/council-scan-inventory.md:141:| `Authorization: Bearer` curl examples | Docs/Config | Existing Guard | Low | High | Bearer auth remains a supported HTTP API auth mechanism and examples use placeholders or `SLSKR_API_TOKEN`, not hard-coded secrets. | Secret scanning and docs freshness gates. |
docs/dev/council-scan-inventory.md:144:| Browser `localStorage` hits | Frontend/API Handling | Existing Guard | Low | High | Token storage regressions are covered by `scripts/check-browser-token-persistence.sh`; remaining production uses are non-secret UI preferences/caches or documented migration fallbacks, with token mentions in tests. | Browser token persistence gate. |
docs/dev/council-scan-inventory.md:146:| SDK `WebSocketClient` examples | Client SDKs | Existing Guard | Low | High | SDK examples route through `WebSocketClient`, whose implementation applies `websocketAuthProtocols(token)`. | TypeScript SDK build/test and WebSocket auth coverage gate. |
docs/INTEGRATION_GUIDE.md:24:The dashboard connects to the API at `http://localhost:8080` (configurable in settings).
docs/INTEGRATION_GUIDE.md:36:("GET", path) if path.starts_with("/api/admin/webhooks/") => handle_get_webhook(path),
docs/INTEGRATION_GUIDE.md:37:("DELETE", path) if path.starts_with("/api/admin/webhooks/") => handle_delete_webhook(path),
docs/INTEGRATION_GUIDE.md:38:("POST", path) if path.ends_with("/test") => handle_test_webhook(path),
docs/INTEGRATION_GUIDE.md:113:    path.to_string(),
docs/INTEGRATION_GUIDE.md:156:    let response = schema.execute(query).await;
docs/INTEGRATION_GUIDE.md:169:slskr-admin api-key create --scopes read write --expires-days 90
docs/INTEGRATION_GUIDE.md:170:slskr-admin api-key list
docs/INTEGRATION_GUIDE.md:171:slskr-admin api-key revoke <id>
docs/INTEGRATION_GUIDE.md:179:slskr-admin webhook create http://example.com/hook --events search.created
docs/INTEGRATION_GUIDE.md:219:cd dashboard && npm run dev  # http://localhost:5173
docs/INTEGRATION_GUIDE.md:222:slskr-admin --api-url http://127.0.0.1:5030 server health
docs/INTEGRATION_GUIDE.md:235:# Forward to port 3000 and navigate to http://localhost:3000
docs/INTEGRATION_GUIDE.md:238:curl http://127.0.0.1:5030/api/health
docs/INTEGRATION_GUIDE.md:245:curl http://127.0.0.1:5030/api/health
docs/INTEGRATION_GUIDE.md:248:curl -H "Authorization: Bearer <key>" http://127.0.0.1:5030/api/admin/api-keys
docs/INTEGRATION_GUIDE.md:251:curl -X POST http://127.0.0.1:5030/api/admin/webhooks \
docs/INTEGRATION_GUIDE.md:253:  -d '{"url": "http://example.com/hook", "events": ["search.created"]}'
docs/INTEGRATION_GUIDE.md:256:curl http://127.0.0.1:5030/api/admin/database/stats
docs/INTEGRATION_GUIDE.md:259:curl -X POST http://127.0.0.1:5030/api/graphql \
docs/INTEGRATION_GUIDE.md:264:curl -i http://127.0.0.1:5030/api/health
docs/INTEGRATION_GUIDE.md:273:curl http://127.0.0.1:5030/api/metrics
scripts/run-slskd-api-compat-smoke.sh:9:api_token="${SLSKR_SLSKD_API_SMOKE_TOKEN:-}"
scripts/run-slskd-api-compat-smoke.sh:10:if [[ -z "$api_token" ]]; then
scripts/run-slskd-api-compat-smoke.sh:29:base_url="http://127.0.0.1:$http_port"
scripts/run-slskd-api-compat-smoke.sh:41:  api_pythonpath="$SLSKD_API_PYTHONPATH"
scripts/run-slskd-api-compat-smoke.sh:43:  api_pythonpath="$work_dir/python"
scripts/run-slskd-api-compat-smoke.sh:44:  "$python_bin" -m pip install --quiet --target "$api_pythonpath" "slskd-api==$api_version"
scripts/run-slskd-api-compat-smoke.sh:52:  export SLSKR_API_TOKEN="$api_token"
scripts/run-slskd-api-compat-smoke.sh:57:  exec target/debug/slskr serve
scripts/run-slskd-api-compat-smoke.sh:61:"$python_bin" - "$base_url" "$api_token" "$api_pythonpath" <<'PY'
scripts/run-slskd-api-compat-smoke.sh:66:base_url, api_token, api_pythonpath = sys.argv[1:4]
scripts/run-slskd-api-compat-smoke.sh:67:sys.path.insert(0, api_pythonpath)
scripts/run-slskd-api-compat-smoke.sh:83:client = SlskdClient(host=base_url, api_key=api_token)
scripts/run-slskd-api-compat-smoke.sh:104:            "token",
scripts/run-slskd-api-compat-smoke.sh:121:        "filename",
scripts/run-slskd-api-compat-smoke.sh:134:            "token",
scripts/run-slskd-api-compat-smoke.sh:193:        "filename",
scripts/run-slskd-api-compat-smoke.sh:239:    return has_keys(value, "filename", "size", "code", "extension", "attributeCount", "attributes")
scripts/run-slskd-api-compat-smoke.sh:278:def is_session_status(value):
scripts/run-slskd-api-compat-smoke.sh:280:        has_keys(value, "expires", "issued", "name", "notBefore", "token", "tokenType")
scripts/run-slskd-api-compat-smoke.sh:284:        and isinstance(value["token"], str)
scripts/run-slskd-api-compat-smoke.sh:285:        and value["tokenType"] == "ApiKey"
scripts/run-slskd-api-compat-smoke.sh:373:    return has_keys(value, "username", "direction", "filename", "state", "exception")
scripts/run-slskd-api-compat-smoke.sh:381:    return has_keys(value, "path", "directory", "count", "totalBytes", "distinctUsers")
scripts/run-slskd-api-compat-smoke.sh:388:record("session.auth_valid", client.session.auth_valid, lambda v: v is True)
scripts/run-slskd-api-compat-smoke.sh:389:record("session.security_enabled", client.session.security_enabled, lambda v: isinstance(v, bool))
scripts/run-slskd-api-compat-smoke.sh:390:record("session.login", lambda: client.session.login("user", "pass"), is_session_status)
scripts/run-slskd-api-compat-smoke.sh:396:identifier = created.get("id") or created.get("token")
scripts/run-slskd-api-compat-smoke.sh:403:record("transfers.enqueue", lambda: client.transfers.enqueue("peer 1", [{"filename": "Remote/Song.mp3", "size": 99}]), lambda v: v is True)
scripts/run-slskd-api-compat-smoke.sh:460:record("relay.download_file", lambda: client.relay.download_file("token"), lambda v: v is True)
scripts/run-slskd-api-compat-smoke.sh:461:record("relay.upload_file", lambda: client.relay.upload_file("token"), lambda v: v is True)
scripts/run-slskd-api-compat-smoke.sh:462:record("relay.upload_share_info", lambda: client.relay.upload_share_info("token"), lambda v: v is True)
scripts/run-slskd-api-compat-smoke.sh:476:if client.transfers.enqueue("telemetry peer", [{"filename": "Telemetry/Album/Track.flac", "size": 321}]) is not True:
scripts/check-kubernetes-public-posture.sh:25:    printf 'kubernetes public posture check failed: expected manifest token missing: %s\n' "$expected" >&2
scripts/check-kubernetes-public-posture.sh:31:  printf 'kubernetes public posture check failed: unsafe/default-deprecated manifest token matched above\n' >&2
scripts/check-shell-script-hygiene.sh:11:if rg -n -g '!check-shell-script-hygiene.sh' 'curl .*\| *(bash|sh)|wget .*\| *(bash|sh)' scripts; then
scripts/check-shell-script-hygiene.sh:12:  printf 'shell script hygiene check failed: network-to-shell execution matched above\n' >&2
scripts/check-shell-script-hygiene.sh:16:if rg -n -g '!check-shell-script-hygiene.sh' 'rm -rf /|git reset --hard|git checkout -- \.' scripts; then
scripts/check-shell-script-hygiene.sh:17:  printf 'shell script hygiene check failed: destructive command pattern matched above\n' >&2
scripts/check-shell-script-hygiene.sh:22:  printf 'shell script hygiene check failed: shell scripts above are not executable\n' >&2
scripts/check-shell-script-hygiene.sh:30:printf 'shell script hygiene check passed\n'
scripts/run-live-soak-proton-natpmp.sh:5:credential_file="${SLSKR_SOAK_CREDENTIAL_FILE:-$repo_root/.secrets/live-soak-account.env}"
scripts/run-live-soak-proton-natpmp.sh:8:if [[ ! -f "$credential_file" && -f "$repo_root/.secrets/pool-listener-account.env" ]]; then
scripts/run-live-soak-proton-natpmp.sh:9:    credential_file="$repo_root/.secrets/pool-listener-account.env"
scripts/run-live-soak-proton-natpmp.sh:68:# shellcheck disable=SC1090
scripts/run-live-soak-proton-natpmp.sh:73:export SLSK_PASSWORD="${SLSK_PASSWORD:-${SLSKR_SOAK_PASSWORD:-${SLSK_INTEGRATION_PASSWORD:?missing soak password}}}"
scripts/run-proton-public-matrix.sh:5:pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
scripts/run-proton-public-matrix.sh:7:    # shellcheck disable=SC1090
scripts/run-proton-public-matrix.sh:10:listener_credential_file="${SLSKR_LISTENER_CREDENTIAL_FILE:-$repo_root/.secrets/live-listener-account.env}"
scripts/run-proton-public-matrix.sh:11:probe_credential_file="${SLSKR_PROBE_CREDENTIAL_FILE:-$repo_root/.secrets/live-probe-account.env}"
scripts/run-proton-public-matrix.sh:19:    [il741]="$repo_root/.secrets/proton-slskr-1.conf"
scripts/run-proton-public-matrix.sh:20:    [au162]="$repo_root/.secrets/proton-slskr-2.conf"
scripts/run-proton-public-matrix.sh:21:    [usca32]="$repo_root/.secrets/proton-slskr-3.conf"
scripts/run-proton-public-matrix.sh:22:    [uk577]="$repo_root/.secrets/proton-slskr-4.conf"
scripts/run-proton-public-matrix.sh:27:        configured_path="${!var_name}"
scripts/run-proton-public-matrix.sh:28:        if [[ "$configured_path" != /* ]]; then
scripts/run-proton-public-matrix.sh:29:            configured_path="$repo_root/$configured_path"
scripts/run-proton-public-matrix.sh:31:        configs[$label]="$configured_path"
scripts/run-proton-public-matrix.sh:38:    local path="$1"
scripts/run-proton-public-matrix.sh:39:    if [[ ! -f "$path" ]]; then
scripts/run-proton-public-matrix.sh:40:        echo "missing required file: $path" >&2
scripts/run-proton-public-matrix.sh:57:    # shellcheck disable=SC1090
scripts/run-proton-public-matrix.sh:69:    # shellcheck disable=SC1090
scripts/run-proton-public-matrix.sh:74:probe_password="$(
scripts/run-proton-public-matrix.sh:76:    # shellcheck disable=SC1090
scripts/run-proton-public-matrix.sh:81:if [[ -z "$probe_username" || -z "$probe_password" ]]; then
scripts/run-proton-public-matrix.sh:123:        SLSK_PASSWORD="$probe_password" \
scripts/run-proton-public-matrix.sh:160:        SLSK_PASSWORD="$probe_password" \
scripts/audit-rust-web-ui.mjs:5:import { extname, join, resolve } from 'node:path';
scripts/audit-rust-web-ui.mjs:6:import { spawnSync } from 'node:child_process';
scripts/audit-rust-web-ui.mjs:12:const repoRoot = resolve(new URL('..', import.meta.url).pathname);
scripts/audit-rust-web-ui.mjs:47:const mockBody = (path) => {
scripts/audit-rust-web-ui.mjs:48:  if (path.includes('/health')) return { service: 'slskr', status: 'ok' };
scripts/audit-rust-web-ui.mjs:49:  if (path.includes('/version')) return { name: 'slskr', version: '0.0.0' };
scripts/audit-rust-web-ui.mjs:50:  if (path.includes('/application')) return { pendingRestart: false, relay: { enabled: false } };
scripts/audit-rust-web-ui.mjs:51:  if (path.includes('/server')) return { isConnected: true, isLoggedIn: true, username: 'audit-user' };
scripts/audit-rust-web-ui.mjs:52:  if (path.includes('/nowplaying')) return { filename: 'audit.flac', peer: 'dj-audit', state: 'playing' };
scripts/audit-rust-web-ui.mjs:53:  if (path.includes('/transfers/speeds')) return { download: 98304, upload: 24576 };
scripts/audit-rust-web-ui.mjs:54:  if (path.includes('/transfers/downloads')) {
scripts/audit-rust-web-ui.mjs:55:    return [{ filename: 'audit-track.flac', peer: 'dj-audit', progress: 0.58, state: 'Active', size: 42152704 }];
scripts/audit-rust-web-ui.mjs:57:  if (path.includes('/transfers/uploads')) {
scripts/audit-rust-web-ui.mjs:58:    return [{ filename: 'shared-track.flac', peer: 'listener-audit', progress: 0.32, state: 'Queued', size: 35127296 }];
scripts/audit-rust-web-ui.mjs:60:  if (path.includes('/searches/1/responses') || path.includes('/searches/:id/responses')) {
scripts/audit-rust-web-ui.mjs:61:    return [{ filename: 'Artist - Audit.flac', username: 'peer-audit', size: 41800000, bitrate: 1011, queue: 0, speed: 512000 }];
scripts/audit-rust-web-ui.mjs:63:  if (path.includes('/searches/records') || path.endsWith('/searches')) {
scripts/audit-rust-web-ui.mjs:66:  if (path.includes('/wishlist')) {
scripts/audit-rust-web-ui.mjs:69:  if (path.includes('/conversations')) {
scripts/audit-rust-web-ui.mjs:72:  if (path.includes('/rooms')) {
scripts/audit-rust-web-ui.mjs:75:  if (path.includes('/users/') && path.includes('/browse')) {
scripts/audit-rust-web-ui.mjs:76:    return { username: 'peer-audit', folders: [{ name: 'Music', files: [{ filename: 'audit.flac', size: 1234 }] }] };
scripts/audit-rust-web-ui.mjs:78:  if (path.includes('/users')) {
scripts/audit-rust-web-ui.mjs:81:  if (path.includes('/contacts')) {
scripts/audit-rust-web-ui.mjs:84:  if (path.includes('/solid/status')) {
scripts/audit-rust-web-ui.mjs:85:    return { enabled: true, webId: 'https://audit.example/profile/card#me', storage: 'ready' };
scripts/audit-rust-web-ui.mjs:87:  if (path.includes('/collections')) {
scripts/audit-rust-web-ui.mjs:90:  if (path.includes('/sharegroups')) {
scripts/audit-rust-web-ui.mjs:93:  if (path.includes('/shared')) {
scripts/audit-rust-web-ui.mjs:96:  if (path.includes('/source-providers')) return [{ id: 'provider-1', name: 'MusicBrainz', enabled: true }];
scripts/audit-rust-web-ui.mjs:97:  if (path.includes('/jobs')) return [{ id: 'job-1', type: 'scan', state: 'Complete' }];
scripts/audit-rust-web-ui.mjs:98:  if (path.includes('/shares')) return { roots: 1, files: 128, scanState: 'Idle' };
scripts/audit-rust-web-ui.mjs:99:  if (path.includes('/database/stats')) return { tracks: 128, peers: 7 };
scripts/audit-rust-web-ui.mjs:100:  if (path.includes('/logs')) return [{ level: 'info', message: 'audit log' }];
scripts/audit-rust-web-ui.mjs:101:  if (path.includes('/telemetry') || path.includes('/metrics')) return { uptimeSeconds: 60 };
scripts/audit-rust-web-ui.mjs:108:      const url = new URL(request.url || '/', 'http://127.0.0.1');
scripts/audit-rust-web-ui.mjs:109:      let filePath = join(distDir, decodeURIComponent(url.pathname));
scripts/audit-rust-web-ui.mjs:110:      if (url.pathname === '/' || !existsSync(filePath)) filePath = join(distDir, 'index.html');
scripts/audit-rust-web-ui.mjs:124:  const build = spawnSync('scripts/build-rust-web.sh', { cwd: repoRoot, stdio: 'inherit' });
scripts/audit-rust-web-ui.mjs:139:  for (const [path, title, slug] of routes) {
scripts/audit-rust-web-ui.mjs:152:      await page.route('**/api/v0/**', async (route) => route.fulfill(json(mockBody(new URL(route.request().url()).pathname))));
scripts/audit-rust-web-ui.mjs:153:      await page.goto(`http://127.0.0.1:${port}${path}`, { waitUntil: 'networkidle' });
scripts/audit-rust-web-ui.mjs:154:      await page.screenshot({ fullPage: true, path: join(auditDir, `${slug}-${viewport.name}.png`) });
scripts/audit-rust-web-ui.mjs:199:        path,
scripts/audit-rust-web-ui.mjs:211:      if (heading !== title) audit.errors.push(`${path} ${viewport.name}: expected heading ${title}, got ${heading}`);
scripts/audit-rust-web-ui.mjs:212:      if (developerOpen) audit.errors.push(`${path} ${viewport.name}: Developer drawer is open by default`);
scripts/audit-rust-web-ui.mjs:213:      if (bodyText.includes('GET /api/v0')) audit.errors.push(`${path} ${viewport.name}: visible raw API text outside Developer drawer`);
scripts/audit-rust-web-ui.mjs:214:      if (primaryActionCount < 1) audit.errors.push(`${path} ${viewport.name}: no primary workflow action`);
scripts/audit-rust-web-ui.mjs:215:      if (nativeWorkspaceCount < 1) audit.errors.push(`${path} ${viewport.name}: missing native workspace`);
scripts/audit-rust-web-ui.mjs:216:      if (inspectorCount < 1) audit.errors.push(`${path} ${viewport.name}: missing inspector/detail surface`);
scripts/audit-rust-web-ui.mjs:217:      if (rowCount < 1) audit.errors.push(`${path} ${viewport.name}: no selectable workflow rows from mocked daemon data`);
scripts/audit-rust-web-ui.mjs:219:        audit.errors.push(`${path} ${viewport.name}: row selection did not update inspector`);
scripts/audit-rust-web-ui.mjs:222:        audit.errors.push(`${path} ${viewport.name}: row selection did not update action status`);
scripts/audit-rust-web-ui.mjs:225:        audit.errors.push(`${path} ${viewport.name}: selected row action did not produce toast/status feedback`);
scripts/audit-rust-web-ui.mjs:228:        audit.errors.push(`${path} ${viewport.name}: main layout does not reserve bottom player space`);
scripts/audit-rust-web-ui.mjs:230:      if (pageErrors.length > 0) audit.errors.push(`${path} ${viewport.name}: browser errors: ${pageErrors.join(' | ')}`);
scripts/run-live-soak-24h.sh:5:credential_file="${SLSKR_SOAK_CREDENTIAL_FILE:-$repo_root/.secrets/live-soak-account.env}"
scripts/run-live-soak-24h.sh:11:# shellcheck disable=SC1090
scripts/run-live-soak-24h.sh:16:export SLSK_PASSWORD="${SLSK_PASSWORD:-${SLSKR_SOAK_PASSWORD:-${SLSK_INTEGRATION_PASSWORD:?missing soak password}}}"
scripts/check-unsafe-blank-opens.sh:8:from pathlib import Path
scripts/check-unsafe-blank-opens.sh:16:    for path in sorted(root.rglob("*")):
scripts/check-unsafe-blank-opens.sh:17:        if path.suffix not in {".js", ".jsx", ".ts", ".tsx"}:
scripts/check-unsafe-blank-opens.sh:19:        lines = path.read_text(encoding="utf-8").splitlines()
scripts/check-unsafe-blank-opens.sh:24:                    print(f"{path}:{index + 1}: _blank link missing rel=\"noopener noreferrer\"", file=sys.stderr)
scripts/check-unsafe-blank-opens.sh:26:            if "window.open(" in line and "safeOpen.js" not in str(path):
scripts/check-unsafe-blank-opens.sh:27:                print(f"{path}:{index + 1}: use safeOpenBlank for _blank window.open calls", file=sys.stderr)
scripts/run-in-proton-wg-netns.sh:111:sudo ip netns exec "$namespace" ip addr add "$ns_ip/24" dev "$ns_veth"
scripts/run-in-proton-wg-netns.sh:112:sudo ip netns exec "$namespace" ip link set "$ns_veth" up
scripts/run-in-proton-wg-netns.sh:113:sudo ip netns exec "$namespace" ip link set lo up
scripts/run-in-proton-wg-netns.sh:114:sudo ip netns exec "$namespace" ip route add default via "$host_ip" dev "$ns_veth"
scripts/run-in-proton-wg-netns.sh:124:sudo ip netns exec "$namespace" ip link add "$wg_name" type wireguard
scripts/run-in-proton-wg-netns.sh:125:sudo ip netns exec "$namespace" ip addr add "$address" dev "$wg_name"
scripts/run-in-proton-wg-netns.sh:129:sudo ip netns exec "$namespace" wg set "$wg_name" private-key "$key_file" \
scripts/run-in-proton-wg-netns.sh:131:sudo ip netns exec "$namespace" ip link set mtu 1420 up dev "$wg_name"
scripts/run-in-proton-wg-netns.sh:132:sudo ip netns exec "$namespace" ip route add "$endpoint_ip/32" via "$host_ip" dev "$ns_veth"
scripts/run-in-proton-wg-netns.sh:133:sudo ip netns exec "$namespace" ip route replace default dev "$wg_name"
scripts/run-in-proton-wg-netns.sh:135:    sudo ip netns exec "$namespace" ip route replace "$extra_route" via "$host_ip" dev "$ns_veth"
scripts/run-in-proton-wg-netns.sh:139:sudo ip netns exec "$namespace" bash -lc 'timeout 3 bash -c "</dev/udp/1.1.1.1/53" 2>/dev/null || true'
scripts/run-in-proton-wg-netns.sh:141:sudo -E ip netns exec "$namespace" runuser --preserve-environment -u "$run_user" -- "$@"
scripts/check-browser-token-persistence.sh:9:if ! rg -n -U 'export const getToken = \(\) =>\s*getSessionStorageItem\(tokenKey\)' web/src/lib/token.js >/dev/null; then
scripts/check-browser-token-persistence.sh:10:  printf 'browser token persistence check failed: web token reader must use sessionStorage only\n' >&2
scripts/check-browser-token-persistence.sh:14:if rg -n -g '!*.test.js' -g '!*.test.jsx' -g '!*.test.ts' -g '!*.test.tsx' 'setToken\(\s*(window\.)?localStorage|localStorage\.setItem\([^)]*slskr-token|rememberMe\s*\?\s*localStorage' web/src dashboard/src; then
scripts/check-browser-token-persistence.sh:15:  printf 'browser token persistence check failed: API token persistence sink matched above\n' >&2
scripts/check-browser-token-persistence.sh:19:if ! rg -n 'sessionStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token|window\.sessionStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token' web/src >/dev/null; then
scripts/check-browser-token-persistence.sh:20:  printf 'browser token persistence check failed: ListenBrainz token must use sessionStorage\n' >&2
scripts/check-browser-token-persistence.sh:24:if rg -n -g '!*.test.js' -g '!*.test.jsx' -g '!*.test.ts' -g '!*.test.tsx' 'localStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token|window\.localStorage\.(setItem|getItem|removeItem)\([^)]*slskr\.listenbrainz\.token' web/src; then
scripts/check-browser-token-persistence.sh:25:  printf 'browser token persistence check failed: ListenBrainz token localStorage sink matched above\n' >&2
scripts/check-browser-token-persistence.sh:30:  printf 'browser token persistence check failed: dashboard apiKey must use session storage\n' >&2
scripts/check-browser-token-persistence.sh:38:printf 'browser token persistence check passed\n'
scripts/run-cross-client-validation.sh:7:extra_env_file="${SLSKR_LIVE_EXTRA_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
scripts/run-cross-client-validation.sh:8:pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
scripts/run-cross-client-validation.sh:77:  tr '\000\n\t' '   ' | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//; s/password=[^ ]+/password=<redacted>/Ig; s/SLSK_PASSWORD=[^ ]+/SLSK_PASSWORD=<redacted>/g; s/SLSKR_SLSK_PASSWORD=[^ ]+/SLSKR_SLSK_PASSWORD=<redacted>/g'
scripts/run-cross-client-validation.sh:118:  local path="${!var_name:-}"
scripts/run-cross-client-validation.sh:119:  if [[ -z "$path" ]]; then
scripts/run-cross-client-validation.sh:123:  if [[ "$path" != /* ]]; then
scripts/run-cross-client-validation.sh:124:    path="$repo_root/$path"
scripts/run-cross-client-validation.sh:126:  if [[ ! -f "$path" ]]; then
scripts/run-cross-client-validation.sh:130:  printf '%s' "$path"
scripts/run-cross-client-validation.sh:196:  local scope="$1" check="$2" expected="$3" actor_index="$4" peer_user="$5" host="$6" filename="$7" expected_text="$8" sha256="$9" private_port="${10:-2240}"
scripts/run-cross-client-validation.sh:200:    SLSK_DOWNLOAD_FILENAME="$filename"
scripts/run-cross-client-validation.sh:280:  find "$commons_fixture_dir" -maxdepth 1 -type f ! -name 'LICENSES.tsv' -exec cp {} "$commons_share/" \;
scripts/run-cross-client-validation.sh:288:  local health_url="http://$http_host:$http_port/health"
scripts/run-cross-client-validation.sh:289:  local app_url="http://$http_host:$http_port/api/v0/application"
scripts/run-cross-client-validation.sh:335:  local dotnet_path
scripts/run-cross-client-validation.sh:336:  dotnet_path="$(command -v dotnet)"
scripts/run-cross-client-validation.sh:362:  password: ${!pass_var}
scripts/run-cross-client-validation.sh:373:      export ASPNETCORE_URLS="http://0.0.0.0:$http_port"
scripts/run-cross-client-validation.sh:387:        "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "$dotnet_path" run --project src/slskr/slskr.csproj --no-launch-profile
scripts/run-cross-client-validation.sh:392:      export ASPNETCORE_URLS="http://0.0.0.0:$http_port"
scripts/run-cross-client-validation.sh:405:      exec "$dotnet_path" run --project src/slskr/slskr.csproj --no-launch-profile
scripts/run-cross-client-validation.sh:559:      run_download_probe_optional slskr-to-slskr download-peer "queued fixture download failed; inspect browse preview for exact remote path and daemon logs for transfer rejection" "$slskr_probe_account_index" "${!slskr_user_var}" "$slskr_host" 'slskr\slskr-interop-slskr.txt' 'slskr interop fixture slskr' a06260a33bda3cf8cb147107c2d09723b4d59fc6a40d1ac9177424614f4f2202 2240
scripts/run-cross-client-validation.sh:561:      run_probe_optional slskr-to-slskr file-transfer-peer "raw transfer token echo requires a queued transfer on slskr; real payload transfer is covered by queued download probes" "$slskr_probe_account_index" "${!slskr_user_var}" env SLSK_FILE_HOST_OVERRIDE="$slskr_host" cargo run -q -p slskr -- probe file-transfer-peer
scripts/run-cross-client-validation.sh:581:      run_download_probe_optional slskr-to-slskr download-peer "queued fixture download failed; inspect browse preview for exact remote path and daemon logs for transfer rejection" "$slskr_probe_account_index" "${!slskr_user_var}" "$slskr_host" 'slskr\slskr-interop-slskr.txt' 'slskr interop fixture slskr' 98be10759b80d65a17fa825c5459338fbd319c338280f7aff65b9cc4bba859a9 2240
scripts/run-cross-client-validation.sh:583:      run_probe_optional slskr-to-slskr file-transfer-peer "raw transfer token echo requires a queued transfer on slskr; real payload transfer is covered by queued download probes" "$slskr_probe_account_index" "${!slskr_user_var}" env SLSK_FILE_HOST_OVERRIDE="$slskr_host" cargo run -q -p slskr -- probe file-transfer-peer
docs/http-api.md:5:The slskr HTTP API provides programmatic access to Soulseek client functionality. All endpoints are available at `/api/v0/*` or `/api/*` paths and require Bearer token authentication for security-sensitive operations.
docs/http-api.md:11:All endpoints require a Bearer token in the `Authorization` header:
docs/http-api.md:14:Authorization: Bearer <token>
docs/http-api.md:17:Tokens are configured in your `slskr.config.toml` file. Request without valid token returns `401 Unauthorized`.
docs/http-api.md:24:Origin: http://localhost:8080
docs/http-api.md:37:- `401 Unauthorized` - Authentication failed or token invalid
docs/http-api.md:99:  "session": {"state": "connected"},
docs/http-api.md:134:    "session-control"
docs/http-api.md:137:    "server-session",
docs/http-api.md:155:List one configured share-root label using virtual share paths only. Local host
docs/http-api.md:156:paths are never included in the response. The default response preserves the
docs/http-api.md:161:- `folder`, `path`, or `prefix` (optional): virtual subfolder under the root.
docs/http-api.md:164:- `q` (optional): case-insensitive virtual path search.
docs/http-api.md:178:      "path": "Track.flac",
docs/http-api.md:179:      "virtual_path": "Music/Artist/Track.flac",
docs/http-api.md:188:      "path": "Album",
docs/http-api.md:189:      "virtual_path": "Music/Artist/Album",
docs/http-api.md:207:directory response shape. Add `/{base64-path}` to list a nested directory.
docs/http-api.md:221:#### `GET /api/sessions`
docs/http-api.md:223:List all active sessions.
docs/http-api.md:228:  "sessions": [
docs/http-api.md:230:      "id": "server-session",
docs/http-api.md:239:#### `POST /api/sessions`
docs/http-api.md:241:Initiate a new session.
docs/http-api.md:251:**Response:** `201 Created` with session details
docs/http-api.md:253:#### `POST /api/sessions/{id}/ping`
docs/http-api.md:255:Send ping to session to keep it alive.
docs/http-api.md:265:#### `DELETE /api/sessions/{id}`
docs/http-api.md:267:Disconnect a session.
docs/http-api.md:273:#### `GET /api/sessions/{id}/privileges`
docs/http-api.md:275:Check user privileges in session.
docs/http-api.md:305:    "token": 1,
docs/http-api.md:334:  "next_token": 1
docs/http-api.md:381:      "filename": "Artist - Song.flac",
docs/http-api.md:467:      "filename": "Artist - Song.flac",
docs/http-api.md:489:  "filename": "Artist - Song.flac"
docs/http-api.md:576:- `folder` (optional): Folder path to browse
docs/http-api.md:577:- `q` (optional): Case-insensitive directory or filename filter
docs/http-api.md:592:          "filename": "Artist/Song.flac",
docs/http-api.md:651:  "folder": "/path/to/share"
docs/http-api.md:733:  "details": "Invalid or missing bearer token"
docs/http-api.md:777:authenticate with the `slskr.api-token.<percent-encoded-token>` subprotocol;
docs/http-api.md:778:non-browser clients may also use the normal bearer authorization path. Polling
docs/http-api.md:840:   api_token = "replace-with-a-random-token"
docs/http-api.md:850:- Use HTTPS in production (reverse proxy with TLS termination)
docs/http-api.md:851:- Rotate bearer tokens regularly
docs/http-api.md:854:- Use strong, randomly-generated bearer tokens
docs/http-api.md:863:- Bearer token usage
docs/http-api.md:867:- Endpoint paths are stable and versioned (`/api/v0/*`)
docs/http-api.md:878:- Compatibility shells that are not active in this runtime keep their endpoint
docs/http-api.md:879:  paths and stable response shapes, but may return empty arrays or
docs/http-api.md:884:  listening-party content helpers, share-grant token/backfill helpers, profile
docs/http-api.md:901:curl http://localhost:8080/api/health
docs/http-api.md:904:curl -H "Authorization: Bearer your-token" \
docs/http-api.md:905:     http://localhost:8080/api/stats
scripts/verify-open-commons-fixtures.sh:16:while IFS=$'\t' read -r id filename media_type size_bytes sha256 license license_url source_url download_url attribution; do
scripts/verify-open-commons-fixtures.sh:21:  path="$dest/$filename"
scripts/verify-open-commons-fixtures.sh:22:  if [[ ! -f "$path" ]]; then
scripts/verify-open-commons-fixtures.sh:23:    echo "missing fixture: $path" >&2
scripts/verify-open-commons-fixtures.sh:27:  actual_size="$(wc -c < "$path")"
scripts/verify-open-commons-fixtures.sh:29:    echo "size mismatch for $filename: expected $size_bytes, got $actual_size" >&2
scripts/verify-open-commons-fixtures.sh:33:  actual_sha="$(sha256sum "$path" | awk '{print $1}')"
scripts/verify-open-commons-fixtures.sh:35:    echo "sha256 mismatch for $filename: expected $sha256, got $actual_sha" >&2
scripts/verify-open-commons-fixtures.sh:39:  if ! grep -Fq "$filename" "$dest/LICENSES.tsv"; then
scripts/verify-open-commons-fixtures.sh:40:    echo "license summary missing fixture: $filename" >&2
scripts/check-python-client-quality.sh:17:import pathlib
scripts/check-python-client-quality.sh:20:root = pathlib.Path("client-python")
scripts/check-python-client-quality.sh:23:for path in sorted(root.rglob("*.py")):
scripts/check-python-client-quality.sh:24:    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
scripts/check-python-client-quality.sh:27:            errors.append(f"{path}:{node.lineno}: bare except is not allowed")
scripts/check-python-client-quality.sh:29:            path_parts = set(path.parts)
scripts/check-python-client-quality.sh:30:            if path.name != "websocket.py" and "examples" not in path_parts and "tests" not in path_parts:
scripts/check-python-client-quality.sh:31:                errors.append(f"{path}:{node.lineno}: library code should not print directly")
scripts/run-proton-natpmp-command.sh:45:exec "$@"
scripts/fetch-open-commons-fixtures.sh:17:printf 'filename\tlicense\tlicense_url\tsource_url\tattribution\n' > "$licenses"
scripts/fetch-open-commons-fixtures.sh:19:while IFS=$'\t' read -r id filename media_type size_bytes sha256 license license_url source_url download_url attribution; do
scripts/fetch-open-commons-fixtures.sh:24:  tmp="$dest/.${filename}.tmp"
scripts/fetch-open-commons-fixtures.sh:25:  out="$dest/$filename"
scripts/fetch-open-commons-fixtures.sh:31:      printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$license" "$license_url" "$source_url" "$attribution" >> "$licenses"
scripts/fetch-open-commons-fixtures.sh:32:      echo "ok $filename"
scripts/fetch-open-commons-fixtures.sh:54:    echo "size mismatch for $filename: expected $size_bytes, got $actual_size" >&2
scripts/fetch-open-commons-fixtures.sh:61:    echo "sha256 mismatch for $filename: expected $sha256, got $actual_sha" >&2
scripts/fetch-open-commons-fixtures.sh:67:  printf '%s\t%s\t%s\t%s\t%s\n' "$filename" "$license" "$license_url" "$source_url" "$attribution" >> "$licenses"
scripts/fetch-open-commons-fixtures.sh:68:  echo "fetched $filename"
scripts/check-package-artifact-matrix.sh:42:    printf 'package artifact matrix check failed: expected packaging token missing: %s\n' "$expected" >&2
scripts/check-package-artifact-matrix.sh:53:    printf 'package artifact matrix check failed: expected SBOM token missing: %s\n' "$expected" >&2
scripts/check-remediation-baseline.sh:9:  scripts/check-browser-token-persistence.sh
scripts/check-remediation-baseline.sh:14:  scripts/check-rate-limit-proxy-policy.sh
scripts/check-remediation-baseline.sh:21:  scripts/check-secret-scanning.sh
scripts/check-remediation-baseline.sh:36:  scripts/check-shell-script-hygiene.sh
scripts/store-live-interop-creds-openbao.sh:6:secret_path="${SLSKR_BAO_SECRET_PATH:-kv/slskr/live-interop/test-accounts}"
scripts/store-live-interop-creds-openbao.sh:38:bao kv put "$secret_path" \
scripts/store-live-interop-creds-openbao.sh:41:  test_1_password="$SLSKR_TEST_1_PASSWORD" \
scripts/store-live-interop-creds-openbao.sh:43:  test_2_password="$SLSKR_TEST_2_PASSWORD" \
scripts/store-live-interop-creds-openbao.sh:45:  test_3_password="$SLSKR_TEST_3_PASSWORD" \
scripts/store-live-interop-creds-openbao.sh:47:  test_4_password="$SLSKR_TEST_4_PASSWORD" \
scripts/store-live-interop-creds-openbao.sh:51:echo "stored live interop credentials at $secret_path"
scripts/start-proton-listener-soak.sh:12:session="${SLSKR_PROTON_SOAK_SESSION:-slskr-live-soak-proton}"
scripts/start-proton-listener-soak.sh:15:active_config="$repo_root/.secrets/${interface}.conf"
scripts/start-proton-listener-soak.sh:18:mkdir -p "$repo_root/target/live-soak" "$repo_root/.secrets"
scripts/start-proton-listener-soak.sh:20:tmux kill-session -t "$session" 2>/dev/null || true
scripts/start-proton-listener-soak.sh:36:tmux new-session -d -s "$session" \
scripts/check-openapi-docs-drift.sh:19:from pathlib import Path
scripts/check-openapi-docs-drift.sh:21:docs_spec_path = Path("docs/openapi.json")
scripts/check-openapi-docs-drift.sh:22:crate_spec_path = Path("crates/slskr/src/openapi.json")
scripts/check-openapi-docs-drift.sh:24:if docs_spec_path.read_bytes() != crate_spec_path.read_bytes():
scripts/check-openapi-docs-drift.sh:29:with docs_spec_path.open(encoding="utf-8") as handle:
scripts/check-openapi-docs-drift.sh:34:if not isinstance(spec.get("paths"), dict) or not spec["paths"]:
scripts/check-openapi-docs-drift.sh:35:    raise SystemExit("docs/openapi.json must contain a non-empty paths object")
scripts/check-openapi-docs-drift.sh:40:    printf 'openapi/docs drift check failed: expected OpenAPI regression token missing: %s\n' "$expected" >&2
scripts/check-openapi-docs-drift.sh:47:    printf 'openapi/docs drift check failed: expected compatibility docs token missing: %s\n' "$expected" >&2
scripts/check-release-version-metadata.sh:31:import pathlib
scripts/check-release-version-metadata.sh:35:root = pathlib.Path.cwd()
scripts/check-release-version-metadata.sh:50:    for dep, version in re.findall(r'(slskr(?:-[a-z]+)?) = \{ version = "([^"]+)", path = "[^"]+" \}', text):
scripts/verify-release-artifacts.sh:64:import pathlib
scripts/verify-release-artifacts.sh:68:destination = pathlib.Path(sys.argv[2]).resolve()
scripts/verify-release-artifacts.sh:72:        member_path = pathlib.PurePosixPath(member.name)
scripts/verify-release-artifacts.sh:73:        if member_path.is_absolute() or ".." in member_path.parts:
scripts/verify-release-artifacts.sh:74:            raise SystemExit(f"unsafe tar entry path: {member.name}")
scripts/verify-release-artifacts.sh:79:        target = (destination / pathlib.Path(*member_path.parts)).resolve()
scripts/verify-release-artifacts.sh:86:import pathlib
scripts/verify-release-artifacts.sh:90:destination = pathlib.Path(sys.argv[2]).resolve()
scripts/verify-release-artifacts.sh:94:        member_path = pathlib.PurePosixPath(member.filename)
scripts/verify-release-artifacts.sh:95:        if member_path.is_absolute() or ".." in member_path.parts:
scripts/verify-release-artifacts.sh:96:            raise SystemExit(f"unsafe zip entry path: {member.filename}")
scripts/verify-release-artifacts.sh:97:        target = (destination / pathlib.Path(*member_path.parts)).resolve()
scripts/verify-release-artifacts.sh:99:            raise SystemExit(f"zip entry escapes destination: {member.filename}")
scripts/verify-release-artifacts.sh:121:    echo "Windows executable present: $root/slskr.exe"
scripts/verify-release-artifacts.sh:123:    echo "archive does not contain slskr executable" >&2
scripts/verify-cargo-package-contents.sh:13:import pathlib
scripts/verify-cargo-package-contents.sh:19:root = pathlib.Path.cwd()
scripts/verify-cargo-package-contents.sh:34:    return [pathlib.Path(line) for line in output.splitlines() if line]
scripts/verify-cargo-package-contents.sh:38:    path = pathlib.PurePosixPath(name)
scripts/verify-cargo-package-contents.sh:39:    return bool(name) and not path.is_absolute() and ".." not in path.parts
scripts/verify-cargo-package-contents.sh:43:    tmp_path = pathlib.Path(tmp)
scripts/verify-cargo-package-contents.sh:44:    workspace_root = tmp_path / "workspace"
scripts/verify-cargo-package-contents.sh:62:                    raise SystemExit(f"package verification failed: unsafe archive path {member.name!r}")
scripts/verify-cargo-package-contents.sh:63:                parts = pathlib.PurePosixPath(member.name).parts
scripts/verify-cargo-package-contents.sh:80:                if relative == pathlib.Path("Cargo.toml"):
scripts/verify-cargo-package-contents.sh:92:                relative = pathlib.PurePosixPath(member.name).relative_to(expected_root)
scripts/verify-cargo-package-contents.sh:93:                target = destination / pathlib.Path(*relative.parts)
scripts/verify-cargo-package-contents.sh:114:homepage = "https://github.com/snapetech/slskr"
scripts/verify-cargo-package-contents.sh:115:repository = "https://github.com/snapetech/slskr"
scripts/verify-cargo-package-contents.sh:131:        ["cargo", "check", "--workspace", "--manifest-path", str(workspace_root / "Cargo.toml")],
scripts/diff-webui-endpoints.sh:35:while IFS=' ' read -r method path; do
scripts/diff-webui-endpoints.sh:38:    # Normalize path for matching (remove query string, replace variables)
scripts/diff-webui-endpoints.sh:39:    norm_path=$(echo "$path" | sed -E 's/\?.*$//' | sed -E 's/:[a-z]+/:var/g' | sed -E 's/\$\{[^}]+\}/:var/g')
scripts/diff-webui-endpoints.sh:40:    if [[ "$norm_path" == /* ]]; then
scripts/diff-webui-endpoints.sh:41:        api_norm_path="/api$norm_path"
scripts/diff-webui-endpoints.sh:43:        api_norm_path="/api/$norm_path"
scripts/diff-webui-endpoints.sh:47:    if [[ "$norm_path" == *":var"* ]]; then
scripts/diff-webui-endpoints.sh:48:        dynamic_prefix="${norm_path%%:var*}"
scripts/diff-webui-endpoints.sh:49:        api_dynamic_prefix="${api_norm_path%%:var*}"
scripts/diff-webui-endpoints.sh:53:    if grep -q "\"$method\".*\"$path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:54:       grep -q "\"$method\".*\"$norm_path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:55:       grep -q "\"$method\".*\"$api_norm_path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:56:       grep -q "\"$method\".*\"/api/v0$norm_path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:57:       grep -q "starts_with.*\"$path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:58:       grep -q "ends_with.*\"$path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:59:       grep -q "path == \"$path\"" "$MAIN_RS" || \
scripts/diff-webui-endpoints.sh:60:       { [[ -n "$api_dynamic_prefix" ]] && grep -q "path_segment_after(path, \"$api_dynamic_prefix" "$MAIN_RS"; } || \
scripts/diff-webui-endpoints.sh:64:        echo "✓ $method $path"
scripts/diff-webui-endpoints.sh:66:        MISSING+=("$method $path")
scripts/diff-webui-endpoints.sh:67:        echo "✗ $method $path"
scripts/check-csp-policy.sh:8:csp_scan_paths=(crates/slskr/src crates/slskr-web/static)
scripts/check-csp-policy.sh:11:  csp_scan_paths+=(web/build/index.html)
scripts/check-csp-policy.sh:14:if rg -n "Content-Security-Policy: .*'unsafe-inline'|script-src .*'unsafe-inline'|style-src .*'unsafe-inline'" "${csp_scan_paths[@]}"; then
scripts/check-csp-policy.sh:20:  printf 'csp policy failed: Rust WASM shell must use an external bootstrap module\n' >&2
scripts/check-csp-policy.sh:30:  printf 'csp policy failed: Rust WASM shell exception must be explicit and scoped\n' >&2
scripts/check-csp-policy.sh:34:wasm_exception_count="$(rg -n "script-src 'self' 'wasm-unsafe-eval'" "${csp_scan_paths[@]}" | awk '!/assert!/ { count++ } END { print count + 0 }')"
scripts/check-rate-limit-proxy-policy.sh:9:if ! rg -n 'SLSKR_TRUSTED_PROXY_CIDRS|trusted_proxy_cidrs' crates/slskr/src docs >/dev/null; then
scripts/check-rate-limit-proxy-policy.sh:10:  printf 'rate-limit proxy policy check failed: trusted proxy CIDR configuration is missing\n' >&2
scripts/check-rate-limit-proxy-policy.sh:14:if ! rg -n 'x_forwarded_for|forwarded_header_client_ip|x_forwarded_for_client_ip' crates/slskr/src/main.rs crates/slskr/src/http_server.rs >/dev/null; then
scripts/check-rate-limit-proxy-policy.sh:15:  printf 'rate-limit proxy policy check failed: Forwarded/X-Forwarded-For parsing is missing\n' >&2
scripts/check-rate-limit-proxy-policy.sh:19:if ! rg -n 'trusted_proxy_cidrs.*any|rate_limit_remote_addr' crates/slskr/src/main.rs >/dev/null; then
scripts/check-rate-limit-proxy-policy.sh:20:  printf 'rate-limit proxy policy check failed: rate limiter must only trust forwarded headers from allowlisted proxies\n' >&2
scripts/check-rate-limit-proxy-policy.sh:25:  printf 'rate-limit proxy policy check failed: spoofing rejection coverage is missing\n' >&2
scripts/check-rate-limit-proxy-policy.sh:30:  printf 'rate-limit proxy policy check failed: BUG-008 must be marked verified in the council ledger\n' >&2
scripts/check-rate-limit-proxy-policy.sh:38:printf 'rate-limit proxy policy check passed\n'
scripts/check-websocket-auth-coverage.sh:21:if ! rg -n 'GET", "/api/events/ws"|websocket_path == "/api/events/ws"' crates/slskr/src/main.rs >/dev/null; then
scripts/check-websocket-auth-coverage.sh:29:elif ! rg -n 'websocketAuthProtocols|slskr\.api-token\.' client-ts/src/websocket-client.ts >/dev/null; then
scripts/check-websocket-auth-coverage.sh:37:elif ! rg -n 'eventFeedProtocols|slskr\.api-token\.' web/src/lib/hubFactory.js web/src/lib/hubFactory.test.js >/dev/null; then
scripts/check-websocket-auth-coverage.sh:42:if ! rg -n 'sec_websocket_protocol|websocket_protocol_authorization|Sec-WebSocket-Protocol' crates/slskr/src/main.rs crates/slskr/src/http_server.rs crates/slskr/src/events_ws.rs >/dev/null; then
scripts/check-council-loop.sh:31:    printf 'council loop check failed: inventory missing required token: %s\n' "$expected" >&2
scripts/check-council-loop.sh:46:    printf 'council loop check failed: required process token missing: %s\n' "$expected" >&2
scripts/check-webhook-outbound-policy.sh:22:for token in 'is_private' 'is_loopback' 'is_link_local' 'is_multicast' '2001:db8' 'SLSKR_WEBHOOK_ALLOW_CIDRS' 'SLSKR_WEBHOOK_DENY_CIDRS' 'localhost' '169.254.169.254'; do
scripts/check-webhook-outbound-policy.sh:23:  if ! rg -n "$token" crates/slskr/src/webhooks.rs >/dev/null; then
scripts/check-webhook-outbound-policy.sh:24:    printf 'webhook outbound policy check failed: expected webhook URL policy/test token missing: %s\n' "$token" >&2
scripts/run-release-gate.sh:36:run_optional_step shellcheck "Shell lint" shellcheck \
scripts/run-release-gate.sh:70:run_step "Smoke web subpath build" node web/scripts/smoke-subpath-build.mjs
scripts/check-remediation-script-registry.sh:24:    printf 'remediation registry failed: %s is not executable\n' "$gate" >&2
scripts/check-transfer-event-growth.sh:21:    printf 'transfer event growth check failed: expected rotation token missing: %s\n' "$expected" >&2
scripts/run-council-scan.sh:51:  -e 'tokio::spawn|spawn\(|abort\(|select!|timeout\(|sleep\(|interval\(|mpsc|broadcast|oneshot' \
scripts/generate-vpn-soulseek-accounts.sh:5:pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
scripts/generate-vpn-soulseek-accounts.sh:7:output_file="${SLSKR_GENERATED_ACCOUNTS_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
scripts/generate-vpn-soulseek-accounts.sh:20:  # shellcheck disable=SC1090
scripts/generate-vpn-soulseek-accounts.sh:25:# shellcheck disable=SC1090
scripts/generate-vpn-soulseek-accounts.sh:40:  local path="${!var_name:-}"
scripts/generate-vpn-soulseek-accounts.sh:41:  if [[ -z "$path" ]]; then
scripts/generate-vpn-soulseek-accounts.sh:45:  if [[ "$path" != /* ]]; then
scripts/generate-vpn-soulseek-accounts.sh:46:    path="$repo_root/$path"
scripts/generate-vpn-soulseek-accounts.sh:48:  if [[ ! -f "$path" ]]; then
scripts/generate-vpn-soulseek-accounts.sh:52:  printf '%s' "$path"
scripts/generate-vpn-soulseek-accounts.sh:73:  password="$(openssl rand -base64 24 | tr -d '\n=+/ ' | cut -c1-24)"
scripts/generate-vpn-soulseek-accounts.sh:80:  SLSK_PASSWORD="$password" \
scripts/generate-vpn-soulseek-accounts.sh:91:      printf 'SLSKR_TEST_%s_PASSWORD=%s\n' "$index" "$(quote_env "$password")"
scripts/generate-vpn-soulseek-accounts.sh:97:    detail="$( { cat "$stdout_file"; tail -n 12 "$stderr_file"; } | tr '\n\t' '  ' | sed -E 's/[[:space:]]+/ /g; s/password=[^ ]+/password=<redacted>/Ig; s/SLSK_PASSWORD=[^ ]+/SLSK_PASSWORD=<redacted>/g' )"
scripts/check-secret-scanning.sh:11:  printf 'secret scanning check failed: BUG-016 must stay verified in council ledger\n' >&2
scripts/check-secret-scanning.sh:15:for ignored in .env web/.env.local .secrets; do
scripts/check-secret-scanning.sh:17:    printf 'secret scanning check failed: expected ignored local secret path is not ignored: %s\n' "$ignored" >&2
scripts/check-secret-scanning.sh:24:import pathlib
scripts/check-secret-scanning.sh:29:root = pathlib.Path.cwd()
scripts/check-secret-scanning.sh:41:    "placeholder", "example", "optional", "changeme", "change-me", "your-", "test-token",
scripts/check-secret-scanning.sh:42:    "secret_generated_value", "spotify-app-client-secret", "live-", "dummy", "redacted",
scripts/check-secret-scanning.sh:44:secret_key = re.compile(r'(?i)\b(api[_-]?key|token|secret|password|passwd|private[_-]?key|client[_-]?secret)\b')
scripts/check-secret-scanning.sh:46:    (?P<key>[A-Z0-9_.-]*(?:api[_-]?key|token|secret|password|passwd|private[_-]?key|client[_-]?secret)[A-Z0-9_.-]*)
scripts/check-secret-scanning.sh:58:def allowed(path: str, key: str, value: str) -> bool:
scripts/check-secret-scanning.sh:59:    haystack = f"{path} {key} {value}".lower()
scripts/check-secret-scanning.sh:66:    if path.endswith("k8s/secrets.example.yaml"):
scripts/check-secret-scanning.sh:68:    if pathlib.Path(path).suffix.lower() == ".md" and len(value) < 64:
scripts/check-secret-scanning.sh:74:    path = root / rel
scripts/check-secret-scanning.sh:75:    if path.suffix.lower() in skip_suffixes or not path.is_file():
scripts/check-secret-scanning.sh:78:        text = path.read_text(encoding="utf-8")
scripts/check-secret-scanning.sh:84:        if not secret_key.search(line):
scripts/check-secret-scanning.sh:90:            if not quote and path.suffix.lower() not in config_like_suffixes:
scripts/check-secret-scanning.sh:95:                findings.append(f"{rel}:{line_no}: possible committed secret in {key}")
scripts/check-secret-scanning.sh:98:    print("secret scanning check failed:", file=sys.stderr)
scripts/check-secret-scanning.sh:108:printf 'secret scanning check passed\n'
scripts/check-proton-wg-labels.sh:5:pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
scripts/check-proton-wg-labels.sh:10:  # shellcheck disable=SC1090
scripts/check-proton-wg-labels.sh:17:  local path="${!var_name:-}"
scripts/check-proton-wg-labels.sh:18:  if [[ -z "$path" ]]; then
scripts/check-proton-wg-labels.sh:21:  if [[ "$path" != /* ]]; then
scripts/check-proton-wg-labels.sh:22:    path="$repo_root/$path"
scripts/check-proton-wg-labels.sh:24:  [[ -f "$path" ]] || return 1
scripts/check-proton-wg-labels.sh:25:  printf '%s' "$path"
scripts/check-proton-wg-labels.sh:45:  if sudo ip netns exec "$namespace" true 2>/dev/null; then
scripts/check-proton-wg-labels.sh:46:    handshake="$(sudo ip netns exec "$namespace" wg show wg0 latest-handshakes 2>/dev/null | awk '{ print $2 }' | head -1)"
scripts/generate-release-manifests.sh:15:import pathlib
scripts/generate-release-manifests.sh:19:root = pathlib.Path.cwd()
scripts/generate-release-manifests.sh:20:out_dir = pathlib.Path(os.environ["OUT_DIR"])
scripts/generate-release-manifests.sh:26:def add_component(ecosystem, name, version, path=None, source=None, license_value=None):
scripts/generate-release-manifests.sh:42:    if path:
scripts/generate-release-manifests.sh:43:        component["evidence"] = {"identity": {"field": "manifest", "confidence": 1.0, "methods": [{"technique": "manifest-analysis", "value": path}]}}
scripts/generate-release-manifests.sh:52:    lock_path = root / "Cargo.lock"
scripts/generate-release-manifests.sh:53:    if not lock_path.exists():
scripts/generate-release-manifests.sh:55:    data = tomllib.loads(lock_path.read_text(encoding="utf-8"))
scripts/generate-release-manifests.sh:67:    path = root / lockfile
scripts/generate-release-manifests.sh:68:    if not path.exists():
scripts/generate-release-manifests.sh:70:    data = json.loads(path.read_text(encoding="utf-8"))
scripts/generate-release-manifests.sh:71:    for package_path, package in sorted(data.get("packages", {}).items()):
scripts/generate-release-manifests.sh:72:        if not package_path or "node_modules/" not in package_path:
scripts/generate-release-manifests.sh:74:        name = package.get("name") or package_path.rsplit("node_modules/", 1)[-1]
scripts/generate-release-manifests.sh:86:    path = root / "client-go/go.sum"
scripts/generate-release-manifests.sh:87:    if not path.exists():
scripts/generate-release-manifests.sh:90:    for line in path.read_text(encoding="utf-8").splitlines():
scripts/generate-release-manifests.sh:102:    path = root / "client-python/setup.py"
scripts/generate-release-manifests.sh:103:    if not path.exists():
scripts/generate-release-manifests.sh:105:    text = path.read_text(encoding="utf-8")
scripts/run-live-http-transfer-smoke.sh:9:  # shellcheck disable=SC1091
scripts/run-live-http-transfer-smoke.sh:15:source_password="${SLSKR_SOURCE_PASSWORD:-${SLSKR_TEST_4_PASSWORD:-${SLSKR_TEST_2_PASSWORD:-}}}"
scripts/run-live-http-transfer-smoke.sh:17:target_password="${SLSKR_TARGET_PASSWORD:-${SLSKR_TEST_2_PASSWORD:-${SLSKR_TEST_1_PASSWORD:-}}}"
scripts/run-live-http-transfer-smoke.sh:19:probe_password="${SLSKR_PROBE_PASSWORD:-${SLSKR_TEST_3_PASSWORD:-}}"
scripts/run-live-http-transfer-smoke.sh:23:  target_password="$SLSKR_TEST_1_PASSWORD"
scripts/run-live-http-transfer-smoke.sh:27:  probe_password="$SLSKR_TEST_1_PASSWORD"
scripts/run-live-http-transfer-smoke.sh:30:if [[ -z "$source_username" || -z "$source_password" || -z "$target_username" || -z "$target_password" || -z "$probe_username" || -z "$probe_password" ]]; then
scripts/run-live-http-transfer-smoke.sh:39:api_token="${SLSKR_LIVE_SMOKE_API_TOKEN:-}"
scripts/run-live-http-transfer-smoke.sh:40:if [[ -z "$api_token" ]]; then
scripts/run-live-http-transfer-smoke.sh:81:  curl -fsS -H "Authorization: Bearer $api_token" "$url"
scripts/run-live-http-transfer-smoke.sh:87:  curl -fsS -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" "$url"
scripts/run-live-http-transfer-smoke.sh:112:fixture_path="$share_dir/$fixture_name"
scripts/run-live-http-transfer-smoke.sh:114:printf '%s\n' "$fixture_payload" >"$fixture_path"
scripts/run-live-http-transfer-smoke.sh:115:fixture_size="$(wc -c <"$fixture_path" | tr -d ' ')"
scripts/run-live-http-transfer-smoke.sh:116:fixture_sha="$(sha256sum "$fixture_path" | awk '{print $1}')"
scripts/run-live-http-transfer-smoke.sh:117:remote_filename="$(basename "$share_dir")/$fixture_name"
scripts/run-live-http-transfer-smoke.sh:141:  export SLSK_PASSWORD="$source_password"
scripts/run-live-http-transfer-smoke.sh:145:  export SLSKR_API_TOKEN="$api_token"
scripts/run-live-http-transfer-smoke.sh:152:  exec cargo run -q -p slskr -- serve
scripts/run-live-http-transfer-smoke.sh:160:  export SLSK_PASSWORD="$target_password"
scripts/run-live-http-transfer-smoke.sh:164:  export SLSKR_API_TOKEN="$api_token"
scripts/run-live-http-transfer-smoke.sh:171:  exec cargo run -q -p slskr -- serve
scripts/run-live-http-transfer-smoke.sh:180:    if session="$(auth_get "http://127.0.0.1:$port/api/v0/session" 2>/dev/null)"; then
scripts/run-live-http-transfer-smoke.sh:181:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" ]]; then
scripts/run-live-http-transfer-smoke.sh:196:wait_session_settled() {
scripts/run-live-http-transfer-smoke.sh:200:  local session seen
scripts/run-live-http-transfer-smoke.sh:202:    if session="$(auth_get "http://127.0.0.1:$port/api/v0/session" 2>/dev/null)"; then
scripts/run-live-http-transfer-smoke.sh:203:      seen="$(printf '%s' "$session" | json_field server_messages_seen 2>/dev/null || echo 0)"
scripts/run-live-http-transfer-smoke.sh:204:      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" && "${seen:-0}" -ge 6 ]]; then
scripts/run-live-http-transfer-smoke.sh:205:        echo "$name session settled messages=$seen"
scripts/run-live-http-transfer-smoke.sh:211:  echo "$name session did not settle: ${session:-no session response}" >&2
scripts/run-live-http-transfer-smoke.sh:215:wait_session_settled source "$source_http_port"
scripts/run-live-http-transfer-smoke.sh:216:wait_session_settled target "$target_http_port"
scripts/run-live-http-transfer-smoke.sh:225:    if listeners="$(auth_get "http://127.0.0.1:$port/api/v0/listeners" 2>/dev/null)"; then
scripts/run-live-http-transfer-smoke.sh:251:      export SLSK_PASSWORD="$probe_password"
scripts/run-live-http-transfer-smoke.sh:253:      exec cargo run -q -p slskr -- probe peer-address
scripts/run-live-http-transfer-smoke.sh:272:    auth_post_json "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse/request" '{}' >/dev/null || true
scripts/run-live-http-transfer-smoke.sh:275:      if browse_json="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse" 2>/dev/null)"; then
scripts/run-live-http-transfer-smoke.sh:279:          echo "target browse path ready files=$count"
scripts/run-live-http-transfer-smoke.sh:290:  echo "target browse path did not become ready before timeout" >&2
scripts/run-live-http-transfer-smoke.sh:299:missing_auth_status="$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$target_http_port/api/v0/config")"
scripts/run-live-http-transfer-smoke.sh:304:auth_get "http://127.0.0.1:$target_http_port/api/v0/config" >/dev/null
scripts/run-live-http-transfer-smoke.sh:307:created="$(auth_post_json "http://127.0.0.1:$target_http_port/api/v0/transfers" "{\"peer_username\":\"$source_username\",\"filename\":\"$remote_filename\",\"size\":$fixture_size}")"
scripts/run-live-http-transfer-smoke.sh:309:auth_post_json "http://127.0.0.1:$target_http_port/api/v0/transfers/$transfer_id/start" '{}' >/dev/null
scripts/run-live-http-transfer-smoke.sh:310:echo "transfer queued id=$transfer_id filename=$remote_filename size=$fixture_size"
scripts/run-live-http-transfer-smoke.sh:315:  last_transfer="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/transfers/$transfer_id")"
scripts/run-live-http-transfer-smoke.sh:339:download_path="$target_state/downloads/$remote_filename"
scripts/run-live-http-transfer-smoke.sh:340:if [[ ! -f "$download_path" ]]; then
scripts/run-live-http-transfer-smoke.sh:341:  echo "downloaded file missing: $download_path" >&2
scripts/run-live-http-transfer-smoke.sh:344:download_sha="$(sha256sum "$download_path" | awk '{print $1}')"
scripts/run-live-http-transfer-smoke.sh:354:  source_state_json="$(auth_get "http://127.0.0.1:$source_http_port/api/v0/session")"
scripts/run-live-http-transfer-smoke.sh:355:  target_state_json="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/session")"
scripts/run-live-http-transfer-smoke.sh:366:auth_get "http://127.0.0.1:$source_http_port/api/v0/stats" >/dev/null
scripts/run-live-http-transfer-smoke.sh:367:auth_get "http://127.0.0.1:$target_http_port/api/v0/stats" >/dev/null
docs/performance-analysis.md:39:- No blocking operations in hot paths
docs/performance-analysis.md:230:    -H "Authorization: Bearer token" \
docs/performance-analysis.md:231:    http://localhost:8080/api/stats
docs/performance-analysis.md:236:- **Zero unsafe code** in hot paths
docs/performance-analysis.md:246:   - Validates on every request (prevents token reuse attacks)
docs/performance-analysis.md:248:   - Benefit: Protection against token leakage
docs/performance-analysis.md:262:- Monitor token usage patterns (detect brute force attempts)
docs/performance-analysis.md:278:- **Stateful endpoints**: Require session affinity or shared state
docs/performance-analysis.md:299:- [Tokio Performance Guide](https://tokio.rs/)
docs/performance-analysis.md:300:- [Rust Performance Guide](https://doc.rust-lang.org/nightly/perf-book/)
docs/performance-analysis.md:301:- [Profiling Rust](https://www.brendangregg.com/perf.html)
docs/legacy-port-harvest.md:7:Keep this repository as canonical. It has the strongest Soulseek network foundation: complete protocol inventory, listener demux, obfuscated type-1 transport, Soulfind contracts, live Proton/NAT-PMP interop, and the current `slskr serve` app shell.
docs/legacy-port-harvest.md:26:   - Redact usernames/passwords/API keys in debug output and HTTP APIs.
docs/legacy-port-harvest.md:30:   - Current app routes now have `/api/v0/*` aliases for health, version, config, shares, session, listeners, and transfers.
docs/legacy-port-harvest.md:31:   - Route contract tests now cover the current read-only JSON shapes, aggregate stats, session command routes including privilege checks, share rescan, versioned aliases, and 404 behavior.
docs/legacy-port-harvest.md:32:   - Bearer-token API auth now protects non-health/version API routes when configured, and non-loopback binds require a token unless auth is explicitly disabled.
docs/legacy-port-harvest.md:37:   - `GET /api/v0/searches/:token`
docs/legacy-port-harvest.md:38:   - `POST /api/v0/searches/:token/complete`
docs/legacy-port-harvest.md:40:   - Model records with token, query, target, active/completed state, timestamps, flattened results, slot/free/queue/speed fields.
docs/legacy-port-harvest.md:52:   - Record direction, peer username, remote virtual path, local target path, expected size, bytes transferred, last error, created/updated timestamps.
docs/legacy-port-harvest.md:53:   - Initial app-state projection exists for create/start/progress/complete/cancel/fail plus stats and list filtering/pagination; real peer transfer execution/resume still needs to be attached.
docs/legacy-port-harvest.md:57:   - `/api/v0/shares/catalog` exists with deterministic sort, virtual path, extension, attribute count, file count, filtered count, total bytes, and `q`/`prefix`/`extension`/`limit`/`offset` filters.
docs/legacy-port-harvest.md:60:   - Keep host absolute paths out of public JSON unless behind an authenticated local-admin view.
docs/legacy-port-harvest.md:68:   - Initial user-watch projection exists for list/watch/unwatch; server watch/unwatch commands, watch/status/stats event projection, and user-stats request command are attached. Browse request/cache projection, failed/partial/indirect-pending-state projection, browse list filtering/pagination, and flattened single-entry or batched `POST /api/v0/browse-responses` ingestion exist; real peer browse execution is attached for direct and indirect peer `GetShareFileList` and folder-content requests.
docs/legacy-port-harvest.md:83:   - Root names should be configured aliases such as downloads/incomplete, not arbitrary absolute paths.
docs/legacy-port-harvest.md:89:   - Useful event names now include search started/completed/pruned, transfer queued/progress/completed/cancelled/failed, share scan completed, user watch/stat/browse/folder-browse requests, message sent/received/acknowledged, room join/leave/message/list requests, and session command requests.
docs/legacy-port-harvest.md:92:   - `GET /api/v0/stats` exists now as a compact operational summary for session, listener, share, search, user, browse, message, room, and transfer projection counts.
docs/legacy-port-harvest.md:94:   - `GET /api/v0/telemetry` exists now as a protected JSON runtime health snapshot for sanitized config flags, session/listener state, storage status, and projection counts.
docs/legacy-port-harvest.md:102:- Web: bind address, port, URL base, static content path, request logging, HTTPS, auth.
docs/legacy-port-harvest.md:103:- Web auth: disabled flag, username/password, JWT, API keys.
docs/legacy-port-harvest.md:104:- Peer network: server address/port, username/password, description, picture, listen address/port, diagnostics, distributed-network options, timeouts, buffers, proxy.
docs/legacy-port-harvest.md:106:- Shares: directories, path filters, cache storage mode, worker count, retention.
docs/legacy-port-harvest.md:136:- Session: `/api/v0/session`, `/api/v0/session/connect`, `/api/v0/session/disconnect`, `/api/v0/session/ping`, `/api/v0/session/privileges/check`
docs/legacy-port-harvest.md:139:- Search: `/api/v0/searches`, `/api/v0/searches/:token`, `/api/v0/searches/:token/complete`, `/api/v0/search-responses`
docs/legacy-port-harvest.md:155:3. Add API contract tests for current health/config/session/listeners/shares/transfers routes.
docs/legacy-port-harvest.md:160:8. Implement transfer execution/resume using the existing `slskr-client` transfer primitives.
docs/install.md:43:Use `SLSKR_CONFIG=/path/to/config.toml` and `SLSKR_STATE_DIR=/path/to/state` to override those paths. Environment variables override config-file values.
docs/install.md:45:Start from [slskr.config.example.toml](./slskr.config.example.toml). Keep credentials and API tokens out of git; use a local ignored env file, service environment file, or secret manager.
docs/install.md:47:SQLite persistence is default-off. Enable the durable compatibility-store path with `SLSKR_PERSISTENCE_ENABLED=true` or `[persistence].enabled = true`; share index, event, search, transfer rows and transfer event trail, user, browse, message, room, collection/library, social/security, OAuth, webhook, and runtime projections write through to SQLite. Transfer projection restart state and event TSV mirrors are also maintained in the slskr state directory.
docs/install.md:53:Spotify supports a browser clickthrough authorization flow. Configure:
docs/install.md:59:# Optional; PKCE/browser authorization does not require a client secret.
docs/install.md:60:# client_secret = "spotify-app-client-secret"
docs/install.md:62:# redirect_uri = "http://127.0.0.1:5030/api/integrations/spotify/callback"
docs/install.md:65:Register the exact redirect URI shown in the WebUI with the Spotify developer dashboard. slskr multiplexes the callback on the existing HTTP listener at `/api/integrations/spotify/callback`; no second callback service or port is required.
docs/install.md:73:- The callback response does not echo the authorization code.
docs/install.md:82:url = "http://127.0.0.1:8686"
docs/install.md:83:api_key = "lidarr-api-key"
docs/install.md:90:Loopback-only HTTP binds default to no API auth unless `SLSKR_API_TOKEN` is configured. Non-loopback binds require an API token unless `SLSKR_AUTH_DISABLED=true` is explicitly set.
docs/install.md:99:http://127.0.0.1:5030/
docs/install.md:102:When API auth is enabled, enter the configured token in the dashboard's browser-session form. API clients can send the same token as:
docs/install.md:105:Authorization: Bearer <token>
docs/install.md:191:Expose the HTTP bind only to the intended network. If exposing outside localhost, set `SLSKR_API_TOKEN`, keep auth enabled, and prefer a reverse proxy that preserves `Host`, `Origin`, and `Referer` headers.
docs/install.md:193:Peer listener ports must match the configured advertised ports. For NAT-PMP/UPnP or VPN forwarded ports, set the advertised regular and obfuscated ports to the public mappings.
docs/install.md:200:- Keep protected API routes behind bearer or same-site browser-cookie auth.
docs/install.md:202:- Do not check in credentials, WireGuard configs, NAT-PMP lease output, cookies, transfer state, share cache, or logs.
docs/GRAPHQL_SCHEMA.graphql:139:  filename: String!
docs/GRAPHQL_SCHEMA.graphql:178:  filename: String!
docs/GRAPHQL_SCHEMA.graphql:278:  path: String!
docs/GRAPHQL_SCHEMA.graphql:371:  filename: String!
docs/open-commons-fixtures.md:29:| `commons-click-track` | `commons-click-track.ogg` | audio/ogg | Public domain | https://commons.wikimedia.org/wiki/File:Audacity_click_track_one_per_second_for_eight_seconds_mono88khz32bitfloat.ogg |
docs/open-commons-fixtures.md:30:| `commons-example-sound` | `commons-example-sound.ogg` | audio/ogg | CC0-1.0 | https://commons.wikimedia.org/wiki/File:Example_sound_file_in_Ogg_Vorbis_format.ogg |
docs/open-commons-fixtures.md:31:| `commons-gif-sample` | `commons-gif-sample.gif` | image/gif | Public domain | https://commons.wikimedia.org/wiki/File:GifSample.gif |
docs/open-commons-fixtures.md:32:| `commons-example-image` | `commons-example-image.png` | image/png | Public domain | https://commons.wikimedia.org/wiki/File:Example_image.png |
docs/dev/bug-council-phases.md:14:- slskNet.Runtime's current canonical analyzer cycle now includes calibrated `CSL0001` through `CSL0016` semantic lenses for allocation, loop-bound, stream-position, file-path, timeout, endpoint, enum/status, slice-bound, diagnostic/log-line, outbound message, cache-key, crypto-trust, dynamic-execution, parser-runtime, resource-capacity, and buffer-operation sinks, plus multi-seed adversarial corpora. The Rust follow-up phases below should use the same calibration rule before treating zero findings as meaningful.
docs/dev/bug-council-phases.md:28:| 4 | Rust loop-bound lens + calibration corpus | Done | (agent) | `scripts/check-rust-protocol-taint-lens.sh` covers protocol-derived loop bounds as well as allocation/read sinks, with bad/good calibration fixtures proving the detector fires and stays quiet on bounded paths. |
docs/dev/bug-council-phases.md:29:| 5 | Multi-seed adversarial protocol corpus | Done | (agent) | `crates/slskr-protocol/tests/adversarial.rs` runs known hostile corpus inputs and multiple deterministic random seeds through frame/message decoders; `scripts/check-rust-protocol-adversarial-corpus.sh` gates the corpus and test execution. |
scripts/check-workflow-release-policy.sh:26:  'id-token: write'; do
scripts/check-workflow-release-policy.sh:28:    printf 'workflow release policy check failed: expected workflow hardening token missing: %s\n' "$expected" >&2
scripts/check-workflow-release-policy.sh:81:  'SLSKR_LIVE_INTEROP_ENV: ${{ secrets.SLSKR_LIVE_INTEROP_ENV }}' \
scripts/check-workflow-release-policy.sh:86:    printf 'workflow release policy check failed: live parity workflow token missing: %s\n' "$expected" >&2
scripts/check-workflow-release-policy.sh:107:    printf 'workflow release policy check failed: release tag policy token missing: %s\n' "$expected" >&2
docs/ENHANCEMENTS.md:102:let secret = Webhook::generate_secret();
docs/ENHANCEMENTS.md:104:    "https://example.com/webhook".to_string(),
docs/ENHANCEMENTS.md:106:    secret.clone(),
docs/ENHANCEMENTS.md:119:let signature = WebhookSignature::create(&payload_bytes, &secret)?;
docs/ENHANCEMENTS.md:122:assert!(signature.verify(&payload_bytes, &secret)?);
docs/ENHANCEMENTS.md:187:    filename TEXT NOT NULL,
docs/ENHANCEMENTS.md:352:    "http://localhost:8080".to_string(),
docs/ENHANCEMENTS.md:405:slskr-admin api-key create --scopes "read" "write" --expires-days 90
docs/ENHANCEMENTS.md:406:slskr-admin api-key list --limit 50
docs/ENHANCEMENTS.md:407:slskr-admin api-key get <id>
docs/ENHANCEMENTS.md:408:slskr-admin api-key revoke <id>
docs/ENHANCEMENTS.md:409:slskr-admin api-key rotate <id>
docs/ENHANCEMENTS.md:426:slskr-admin webhook create http://example.com/hook --events search.created transfer.started
docs/ENHANCEMENTS.md:461:--api-url http://localhost:8080    # API server URL
docs/ENHANCEMENTS.md:462:--api-key <key>                    # API authentication key
docs/dev/bug-council-severity-schema.md:12:| Low | Defensive-depth gap: code path is currently unreachable from untrusted input, but the absence of the guard is itself a hazard if a refactor exposes it. |
docs/dev/bug-council-severity-schema.md:15:Pick the **worst plausible** severity given current code paths. If the same code is reachable from two boundaries with different severities, take the higher.
docs/openapi.json:9:      "url": "https://github.com/snapetech/slskr"
docs/openapi.json:13:      "url": "https://www.gnu.org/licenses/agpl-3.0.html"
docs/openapi.json:18:      "url": "http://localhost:8080",
docs/openapi.json:22:      "url": "https://api.example.com",
docs/openapi.json:31:  "paths": {
docs/openapi.json:253:            "in": "path",
docs/openapi.json:471:            "in": "path",
docs/openapi.json:502:            "in": "path",
docs/openapi.json:734:          "token": {
docs/openapi.json:807:          "next_token": {
docs/openapi.json:811:        "required": ["entries", "count", "filtered_count", "offset", "next_token"]
docs/openapi.json:837:          "filename": {
docs/openapi.json:850:        "required": ["username", "filename", "size"]
docs/openapi.json:970:          "filename": {
docs/openapi.json:990:          "filename",
docs/openapi.json:1005:          "filename": {
docs/openapi.json:1009:        "required": ["direction", "peer_username", "filename"]
docs/openapi.json:1021:          "path": {
docs/openapi.json:1028:        "required": ["id", "method", "path"]

## Public mutable ownership surfaces
