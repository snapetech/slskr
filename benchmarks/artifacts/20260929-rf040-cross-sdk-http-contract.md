# RF-040 shared SDK HTTP contract fixture (2026-09-29 UTC)

Internal-only test evidence. No SDK runtime behavior changed and no release was
published.

`testdata/sdk-http-contract.json` is read by the Go, Python, and TypeScript
client suites. Each suite uses an ephemeral loopback HTTP server to exercise the
same structured HTTP 422 response, a GET connection dropped before headers, and
a POST connection dropped before its response.

| Case | Go | Python | TypeScript |
| --- | --- | --- | --- |
| Structured HTTP 422 | Status 422, `validation_failed`, and `query is required` details | Same status, code, and details | Same status, code, and details |
| GET after one dropped connection | One attempt, returns the transport error | Two attempts, second response succeeds | Two attempts, second response succeeds |
| POST after a dropped response, retry budget 3 | One attempt | One attempt | One attempt |

The live loopback cases and the existing SDK suites pass under
`scripts/check-client-sdk-gates.sh`: Go client tests, 50 Python tests plus
package checks, and 52 TypeScript tests plus lint/build. The repo gate completed
in 18.7 seconds with a 386.6 MiB memory peak and zero swap. Temporary HTTP
servers and the Python environment were closed by the tests and gate.

The different GET behavior is intentional and now executable evidence: Python
and TypeScript retry transport failures for idempotent reads, while Go exposes
the transport failure directly. None of the clients replays a mutation after an
uncertain response.
