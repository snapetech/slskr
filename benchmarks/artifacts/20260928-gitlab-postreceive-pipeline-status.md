# RF-008/RF-067 GitLab post-receive investigation (2026-09-28)

Internal-only operational diagnostic. No product behavior or GitLab service
configuration was changed during this investigation.

## Findings

- GitLab accepted `main` through `4cc18c269fb741b98f15c7b9207405c02b274ea6`;
  `git ls-remote gitlab refs/heads/main` returned that exact commit.
- GitLab's internal `allowed` and `post_receive` endpoints returned HTTP 200
  for pushes through `b9679f30` and for the `b9679f30` to `4cc18c26` push at
  2026-09-28 22:39 UTC.
- A read-only query of GitLab project 44's pipeline partitions still showed
  pipeline #81 as the newest record, created 2026-05-17. No pipeline record
  exists for the accepted current `main` tip.
- The project has CI enabled (`builds_access_level=20`) and no custom CI path,
  so its repository `.gitlab-ci.yml` is the configured source.
- The configured GitLab MCP token is present but the GitLab API returns 401.
  A direct Rails runner query exceeded its 75-second bound and was terminated;
  short PostgreSQL and log queries completed normally.
- `Repositories::PostReceiveWorker` logs repeated exclusive-lease conflicts
  for `DetectRepositoryLanguagesWorker`. No evidence ties those messages to
  pipeline creation, so they remain a diagnostic lead rather than a cause.
- GitLab services remained healthy; none was restarted and no Redis or project
  settings were changed.

RF-008 and RF-067 remain open pending a GitLab pipeline being created and
running on a post-repair commit.
