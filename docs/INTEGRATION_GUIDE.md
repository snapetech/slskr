# Historical Integration Guide

Status: archived planning material. This file is retained for migration
context; it is not an implementation checklist or an operator runbook.

The previous version mixed completed modules with proposed routes, unshipped
GraphQL and CLI examples, and deployment instructions that do not describe the
current product. Those examples have deliberately been removed so this archive
cannot be mistaken for supported behavior.

## Current sources of truth

Use these maintained documents and source locations when integrating with
slskR:

- [Install guide](install.md) covers build, service, configuration, state, and
  exposure.
- [App surface](app-surface.md) lists the supported CLI, daemon, HTTP, auth,
  and Web UI behavior.
- [HTTP API reference](http-api.md) is the maintained route and payload
  reference.
- [HTTP API deployment](http-api-deployment.md) covers authentication,
  proxies, exposure, and Kubernetes deployment.
- [Webhook API](WEBHOOK_API.md) is the current webhook contract, including
  signing, retries, and outbound-target policy.
- [Web UI README](../web/README.md) documents the shipped React/Vite browser
  application and its build and audit gates.
- [Client libraries](CLIENT_LIBRARIES.md) and the individual client READMEs
  document supported automation clients.

## Archived topic map

The old guide discussed these areas. Their current status must be determined
from the sources above and the implementation, not from the former snippets:

| Topic | Current reference |
| --- | --- |
| Request tracing and correlation IDs | `crates/slskr/src/tracing.rs` and the daemon route middleware. |
| Webhooks | `docs/WEBHOOK_API.md` and the `/api/webhooks` and `/api/admin/webhooks` routes. |
| SQLite persistence | `docs/install.md`, `docs/app-surface.md`, and `crates/slskr/src/persistence.rs`. |
| GraphQL | `docs/GRAPHQL_SCHEMA.graphql` contains contract/design notes; the maintained HTTP API reference does not advertise a live GraphQL endpoint. |
| Browser UI | `web/` is the shipped browser bundle served by `slskr serve`; use its README and the app-surface documentation. |
| Standalone dashboard package | `dashboard/` remains in repository CI coverage, but it is not the browser bundle served by `slskr serve`. |
| Administrative automation | Use the documented HTTP API and the supported Go, Python, and TypeScript clients; the old `slskr-admin` command examples are archival. |

## How to add a real integration

A new supported integration is not complete when a module or schema exists.
It needs an implemented route or runtime hook, bounded and authenticated
behavior where applicable, focused tests, documentation in the maintained
references, and the required release-note fragment. Update the relevant
maintained document and parity ledger when the behavior is actually shipped.
