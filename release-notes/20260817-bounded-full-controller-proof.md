---
category: changed
audience: operators
area: build-and-test-memory
action: none
breaking: false
---

The historical full controller differential suite can now be explicitly
audited only inside the hard process-memory guard. Ordinary and unguarded
requests remain refused, so an evidence run cannot allocate without the
repository memory ceiling.
