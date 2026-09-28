---
category: changed
audience: operators
area: ci
action: none
breaking: false
---
The TypeScript SDK gate builds once through the package check, which verifies tracked distribution output and package contents. This removes the duplicate build in the combined SDK checks.
