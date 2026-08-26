---
category: security
audience: operators
area: release-pipeline
action: none
breaking: false
---
Rust and process-memory guard nesting markers no longer bypass resource limits, serialization, or protected test-profile checks when supplied from the environment.
