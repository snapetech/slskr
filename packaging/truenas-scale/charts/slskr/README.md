# slskR TrueNAS SCALE chart

The chart provides a non-root Deployment, a Web UI Service on port 5030, and
persistent storage for configuration and transfers. Render it with Helm or
install it through the TrueNAS SCALE catalog tooling.

For a direct Helm deployment:

```sh
helm install slskr ./packaging/truenas-scale/charts/slskr
helm upgrade slskr ./packaging/truenas-scale/charts/slskr
```
