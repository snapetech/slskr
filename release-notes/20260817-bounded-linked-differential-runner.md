---
category: changed
audience: operators
area: parity-certification
action: none
breaking: false
---

Parity certification now executes the historical controller, persistence,
file-lifecycle, protocol, security-control, and security-authorization proofs
through bounded linked runners. The audit also auto-selects an installed
Chromium executable when the Playwright-managed browser is absent; all proof
processes remain under the repository's hard memory guard.
