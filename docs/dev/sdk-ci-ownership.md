# SDK CI ownership

This repository has one combined SDK source-of-truth gate:
`scripts/check-client-sdk-gates.sh`. It runs the Go tests, Python quality and
package checks, and the TypeScript test, lint, advisory, build, and package
checks used by the release/remediation paths and the GitHub workflow.

The GitLab jobs are intentionally split by native image rather than invoking
the combined gate in an image that cannot provide all three toolchains:

| Job | Owns | Deliberately does not repeat |
| --- | --- | --- |
| `go:client` | Read-only Go module test on the Go image | Python/Node SDK checks |
| `node:client-ts` | Fast Node lint/build feedback on the Node image | TypeScript tests, audit, and package reproducibility already owned by the combined gate |

The GitHub and release paths call the combined gate once. The GitLab jobs are
platform-specific feedback lanes, not an additional source of release
certification.
