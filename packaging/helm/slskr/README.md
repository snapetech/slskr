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
downloads, incomplete transfers, shares, and daemon state in persistent
volumes. The image and chart run as UID/GID 1000 so mounted volumes remain
writable with the default security context.
