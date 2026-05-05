# slskr HTTP API Deployment

This page is current release guidance. Older `http_api_*` and `SLSKR_BEARER_TOKEN` names are obsolete.

## Local Default

`slskr serve` binds to `127.0.0.1:5030` by default. Loopback-only binds may run without API auth when no token is configured.

```bash
SLSKR_HTTP_BIND=127.0.0.1:5030 cargo run -p slskr -- serve
curl http://127.0.0.1:5030/api/health
```

## Exposed Deployment

Any non-loopback bind requires `SLSKR_API_TOKEN` unless `SLSKR_AUTH_DISABLED=true` is explicitly set. Do not disable auth on exposed binds.

```bash
export SLSKR_HTTP_BIND=0.0.0.0:5030
export SLSKR_API_TOKEN="$(openssl rand -hex 32)"
export SLSKR_AUTH_DISABLED=false
slskr serve
```

Protected routes accept:

- `Authorization: Bearer <token>`
- `X-API-Key: <token>`
- `slskr.session` cookie only when `SLSKR_API_COOKIE_AUTH_ENABLED=true`

Health/version/capability bootstrap routes remain public: `/`, `/api/health`, `/api/version`, `/api/session/enabled`, and `/api/v0/capabilities`.

## Reverse Proxy

Terminate TLS at the proxy and forward to the loopback-bound daemon where possible.

```nginx
server {
    listen 443 ssl http2;
    server_name slskr.example.com;

    ssl_certificate /etc/letsencrypt/live/slskr.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/slskr.example.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:5030;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header Forwarded "for=$remote_addr;proto=$scheme;host=$host";
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}
```

Set `SLSKR_TRUSTED_PROXY_CIDRS` or `[auth].trusted_proxy_cidrs` to the proxy source CIDRs before relying on `Forwarded` or `X-Forwarded-For` for anonymous rate-limit keys. slskr ignores forwarded client IP headers from untrusted peers so direct clients cannot spoof another address.

Do not add wildcard `Access-Control-Allow-Origin: *` for authenticated browser deployments. slskr emits same-origin CORS responses and rejects cross-site mutating API requests when auth is enabled.

## Kubernetes

Use the maintained manifests under `k8s/`. They run the API as one stateful replica by default, mount `/data` from the `slskr-data` PVC, and expose metrics at `/api/metrics` on the authenticated HTTP port.

Before applying, create real secrets from placeholders in `k8s/secrets.example.yaml`:

```bash
kubectl create namespace slskr
kubectl -n slskr create secret generic slskr-secrets \
  --from-literal=SLSKR_API_TOKEN="$(openssl rand -hex 32)" \
  --from-literal=SLSK_USERNAME="<username>" \
  --from-literal=SLSK_PASSWORD="<password>"
```

Then set concrete release image tags in `k8s/deployment.yaml` and apply:

```bash
kubectl apply -k k8s
kubectl -n slskr rollout status deployment/slskr-api
```

## Metrics

Prometheus-compatible metrics are served at both `/api/metrics` and `/api/v0/metrics`. These routes are protected when API auth is enabled. The Kubernetes ServiceMonitor uses `/api/metrics` on the `http` service port and reads `SLSKR_API_TOKEN` from `slskr-secrets` as a bearer token.
