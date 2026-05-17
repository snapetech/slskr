# On-demand Windows runner

The `Windows Smoke` workflow uses the private `snapetech/packer` Windows VM
runner:

```yaml
runs-on: [self-hosted, Windows, X64, packer-windows]
```

This gives Windows Rust/WASM/web build coverage without keeping a Windows VM
running. The runner is ephemeral and powers down after one job.
