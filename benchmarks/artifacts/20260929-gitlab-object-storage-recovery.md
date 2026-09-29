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
