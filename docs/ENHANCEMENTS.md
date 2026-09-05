# Historical Enhancement Inventory

Status: archived planning material. This document is not a release claim,
implementation checklist, or deployment guide. The former version described
planned integration work as complete and included examples for routes and
commands that are not part of the supported product surface.

Use the maintained documentation instead:

- [Documentation index](README.md)
- [App surface](app-surface.md)
- [HTTP API reference](http-api.md)
- [HTTP API deployment](http-api-deployment.md)
- [Webhook API](WEBHOOK_API.md)
- [Install guide](install.md)
- [Web UI README](../web/README.md)
- [Client libraries](CLIENT_LIBRARIES.md)

## Topic map

This table preserves the historical areas without asserting that an example in
the old document is a supported endpoint, command, or deployment recipe.

| Historical topic | Current source of truth |
| --- | --- |
| Request tracing and correlation IDs | `crates/slskr/src/tracing.rs` and daemon route middleware. |
| Webhook subscriptions and HMAC delivery | `docs/WEBHOOK_API.md` and the implemented webhook routes. |
| SQLite persistence | `docs/install.md`, `docs/app-surface.md`, and `crates/slskr/src/persistence.rs`. |
| GraphQL schema | `docs/GRAPHQL_SCHEMA.graphql` is design/contract material; the maintained HTTP API reference does not advertise a live GraphQL endpoint. |
| Benchmarks and performance work | `docs/performance-analysis.md` and the current repository scripts/tests. |
| Administrative clients | `docs/CLIENT_LIBRARIES.md` and the Go, Python, and TypeScript client READMEs. |
| Kubernetes and release packaging | `docs/http-api-deployment.md` and `docs/release.md`. |
| Browser administration UI | `web/` is the shipped bundle; `dashboard/` is retained in repository CI coverage as a separate package. |

Historical line counts, test totals, “production ready” labels, and command
examples are intentionally omitted because they drift and previously caused
this archive to report unsupported behavior as available functionality.

When a future enhancement is shipped, document its actual route or user
behavior in the maintained references, add focused coverage, and add the
required release-note fragment. Keep speculative work in a dated plan or
parity ledger rather than marking it complete here.
