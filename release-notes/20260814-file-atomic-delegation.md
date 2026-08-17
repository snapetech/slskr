---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

The parity file-lifecycle audit now composes the executed atomic-writer
contract into frozen slskdN services that delegate their durable writes to the
same atomic temp-file replacement primitive and expose a matching load path.
Direct transfer, relay, secure-path, and generated-certificate writers remain
separate audit subjects.
