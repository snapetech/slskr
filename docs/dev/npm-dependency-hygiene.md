# npm dependency hygiene

This is the checked-in exception record for frontend package maintenance. The
lockfiles are authoritative; `node_modules` directories are disposable and are
not evidence of the release dependency graph.

## Current baseline

- `web` no longer depends on `eslint-config-canonical`. The web ESLint config
  imports `@vitest/eslint-plugin` and `eslint-plugin-react` directly, so an
  unused lint preset cannot pull its deprecated transitive graph back in.
- The web and dashboard lockfiles contain no deprecated package entries.
- Jest's `test-exclude` range still names the deprecated glob 10 line. The
  `client-ts/package.json` overrides Jest's test-exclude glob to `^13.0.6`,
  the same supported line used by Jest itself, so no deprecated package entry
  remains in the checked-in lockfiles. This is checked by
  `scripts/check-npm-dependency-hygiene.sh`.
- `scripts/check-web-audit.sh` audits all three frontend graphs. It retries the
  registry path and falls back only to a validated offline npm audit report;
  CI and release gates therefore keep the same behavior when the advisory
  endpoint is unavailable. The graph is acceptable only while the advisory
  gate reports no unexpected moderate-or-higher vulnerability.

## Modernization track

The current supported baseline is kept on the lockfile-resolved patch releases
that pass the existing UI and SDK gates. Major upgrades are isolated so they
cannot be hidden inside a routine dependency refresh:

1. Upgrade TypeScript 6 to 7 in `web`, `dashboard`, and `client-ts` together;
   run the three builds, dashboard type-check, lint, and SDK declaration
   checks, then review generated declarations and compiler diagnostics.
2. Upgrade dashboard Vitest 4 to 5 with its testing-library adapters; run the
   dashboard suite and the web suite because both share React test helpers.
3. Re-run the web/dashboard production builds, client SDK build, advisory
   audits, and release package manifest generation before changing the tracked
   baseline.
4. Keep this document and the ledger entry current with the exact compatibility
   result. A major upgrade is not considered complete merely because npm
   resolves it.

The TypeScript 7 upgrade is not included in this batch: the current
`typescript-eslint` and `ts-jest` releases reject TypeScript 7 through their
peer ranges. The supported TypeScript 6 baseline stays in place until those
upstream parsers support TypeScript 7. Dashboard Vitest 5 is included only if
its type-check, lint, and test gates pass.
