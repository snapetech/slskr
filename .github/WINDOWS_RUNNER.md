# Windows runners

Windows coverage uses GitHub-hosted `windows-latest` runners. The `CI`
platform matrix builds and archive-verifies the native
`x86_64-pc-windows-msvc` release artifact and runs the workspace tests. The
separate `Windows Smoke` workflow remains available for on-demand or
pull-request Rust/WASM/web smoke coverage.

The release workflow uses the same hosted runner for the Windows archive. No
self-hosted Windows labels or private VM are required.
