# Gold Star Club

The Gold Star Club is a reserved PodCore community and testing pod. It is not
a download entitlement or a separate Soulseek account tier.

## Intent

The pod was originally conceived as a small channel where testers could submit
feedback about slskr and reference-client behavior. Discord is now the primary feedback
channel, so the club is not the supported route for general support. The
reserved pod and its `testing`/`realm-governance` tags remain as a bounded test
surface and as a hook for future realm-based group governance. That future
governance is not currently used to grant permissions or benefits.

## Current behavior

When enabled, slskR creates the reserved pod in `pods.json` and auto-joins the
local Soulseek identity after it connects:

- The pod is public, has one `General` channel, does not admit guests, and does
  not require approval.
- Membership is capped at 250 active members.
- Leaving the pod is a local, irreversible opt-out. slskR writes
  `gold-star-club.revoked` in the state directory and will not rejoin that
  local instance on a later restart.
- Disabling auto-join hides the reserved pod and blocks its create/join paths;
  it does not delete ordinary pod state. A local revocation remains in force
  even if auto-join is enabled again.

## Configuration

The setting is read at startup. Environment variables override TOML values.

```toml
[podcore.gold_star_club]
autojoin = false
```

The slskr environment name is:

```bash
SLSKR_POD_GOLD_STAR_CLUB_AUTOJOIN=false
```

The default depends on the selected behavior profile:

| Profile | Default | To enable or disable |
| --- | --- | --- |
| Native/current slskr (`SLSKR_PARITY_PROFILE=current`, or no explicit runtime profile) | Opt-in | Set `autojoin = true` or the environment variable to `true`. |
| Frozen legacy profile (`SLSKR_PARITY_PROFILE=frozen`, or an explicit `SLSKR_CONTROLLER_PROFILE` when the parity profile is not `current`) | Enabled | Set `autojoin = false` or the environment variable to `false`. |

The environment variable accepts the usual boolean spellings (`true`/`false`,
`1`/`0`, `yes`/`no`, and `on`/`off`).
