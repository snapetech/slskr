# slskR Helm chart

Install the chart with:

```sh
helm install slskr ./packaging/helm/slskr
```

Upgrade an existing release with:

```sh
helm upgrade slskr ./packaging/helm/slskr
```

The chart exposes the Web UI on service port 5030 and stores configuration,
downloads, incomplete transfers, and shares in persistent volumes.
