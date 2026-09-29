# RF-008/RF-067 GitLab Object Storage Recovery Check (2026-09-29 UTC)

Internal-only operational and CI diagnostic. It documents test/development
dependency maintenance and GitLab recovery evidence; it does not describe a
user-facing product change. No GitLab configuration or object-store database
markers were left changed by the investigation.

## Dashboard audit

GitHub CI run `36508137976` on `ec415810` failed the dashboard npm audit gate
because `jsdom@30.0.1` resolved vulnerable transitive `undici@8.10.1`. The
dashboard lock now pins the first patched compatible release, `undici@8.10.2`.
The direct `scripts/check-web-audit.sh dashboard` gate reports zero
vulnerabilities. The fix is an internal dashboard test/development dependency
update; hosted CI rerun is still required.

The advisory lists `undici@8.10.2` as patched:
<https://github.com/nodejs/undici/security/advisories/GHSA-3wwx-pv8p-q78v>.

## GitLab post-receive and object store

- GitLab CE 18.9.2 accepts repository pushes, but project 44 has no pipeline
  newer than pipeline #81 from 2026-05-17. The GitLab API token configured for
  the MCP integration returns HTTP 401.
- `Repositories::PostReceiveWorker` fails while recording user activity with
  `Object Storage is not enabled for AvatarUploader`. The current omnibus
  configuration disables the consolidated object store because its former
  MinIO endpoint belonged to the retired k3s cluster.
- Three avatar uploads and 57 job artifact rows still reference remote object
  storage. A bounded supported migration attempt changed no database markers:
  the first artifact read timed out against the unavailable endpoint.
  The original GitLab configuration was restored, Puma and the runner were
  brought back, and GitLab health returned to `GitLab OK`.
- Velero backup `cluster-backup-no-ollama-20260512020041` contains the GitLab
  MinIO deployment and PVC metadata, but its `volumeinfo` marks the data backup
  skipped because the node backup agent was absent on `kspld0`. The recorded
  local PV path is absent on that node. This backup therefore cannot restore
  the MinIO objects.
- No SQL marker was changed to claim unavailable remote objects were local, and
  no object was deleted or fabricated. Existing avatar/artifact object content
  remains unrecovered; GitLab push-triggered pipeline proof remains open.

## Superseding storage and pipeline status (2026-09-29 UTC)

The preceding section records the pre-recovery state. GitLab's consolidated
object store is now enabled against the SeaweedFS S3 endpoint, with artifacts
written to the configured `gitlab-artifacts` bucket. An authenticated SigV4
probe from `gitlab-external` wrote an object, read back identical bytes, and
deleted it successfully (HTTP 200/200/204). The probe object was removed.

The existing GitLab PAT `snape-gitlab-mcp-admin-2026-09` was rotated in place.
The protected local configuration files are mode `0600`; authenticated
`GET /api/v4/user` returns the expected admin account. The already-running
GitLab MCP process still has its previous token cached and returns 401; a newly
started MCP process will read the rotated value. No token value is stored here.

GitLab pipeline 141 on `af16a5ad` passed its Rust, Go, Web, dashboard,
TypeScript, and `github:mirror` jobs, demonstrating that default-branch pushes
schedule and complete CI again. Pipeline 142 on exact source `1dcbb95a` passed
all six jobs: Rust, Go, Web, dashboard, TypeScript, and `github:mirror`. This
also verifies the retention-shutdown cancellation change on a clean GitLab
runner and confirms GitLab mirrored the same branch tip to GitHub.

The original MinIO data volume is still absent: the available Velero backup
explicitly skipped its volume data. Historical avatar and artifact object
bytes therefore remain unrecovered. New pipeline artifacts are being written
to the active object store; this does not reconstruct the old objects.
