# SoulseekR Kubernetes Deployment Guide

This guide provides complete instructions for deploying the production-ready slskr ecosystem to Kubernetes.

## Prerequisites

- Kubernetes 1.24+
- kubectl configured with cluster access
- Helm 3.0+ (optional, for package management)
- cert-manager (for TLS certificates)
- Prometheus Operator (for metrics)
- Nginx Ingress Controller

## Quick Start Deployment

### 1. Build and Push Docker Image

```bash
# Build the production binary
cargo build --release

# Create Dockerfile
cat > Dockerfile << 'EOF'
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/slskr /usr/local/bin/
COPY --from=builder /app/dashboard/dist /var/www/dashboard

EXPOSE 8080 9090
ENTRYPOINT ["slskr"]
EOF

# Build and push image
docker build -t your-registry/slskr:latest .
docker push your-registry/slskr:latest
```

### 2. Deploy to Kubernetes

```bash
# Create namespace and deploy
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/rbac.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/hpa.yaml
kubectl apply -f k8s/pdb.yaml
kubectl apply -f k8s/ingress.yaml
kubectl apply -f k8s/servicemonitor.yaml

# Or use Kustomize
kubectl apply -k k8s/
```

### 3. Verify Deployment

```bash
# Check deployment status
kubectl get deployment -n slskr
kubectl get pods -n slskr

# Check service endpoints
kubectl get svc -n slskr

# View logs
kubectl logs -n slskr -l app=slskr-api -f

# Access API health check
kubectl port-forward -n slskr svc/slskr-api 8080:8080
curl http://localhost:8080/api/health

# Access dashboard
kubectl port-forward -n slskr svc/slskr-dashboard 3000:80
# Open http://localhost:3000 in browser
```

## Deployment Architecture

### Components

1. **slskr-api** (3+ replicas)
   - Main HTTP API server
   - Handles searches, transfers, messages, webhooks
   - Metrics endpoint at `/api/metrics`
   - Resource limits: CPU 250m-1000m, Memory 256Mi-512Mi

2. **slskr-dashboard** (2-5 replicas)
   - React admin dashboard
   - Real-time monitoring and management
   - Connected to API at `http://localhost:8080`

3. **Horizontal Pod Autoscaling**
   - API scales 3-10 replicas based on CPU (70%) / Memory (80%)
   - Dashboard scales 2-5 replicas based on CPU (80%)
   - Quick scale-up, gradual scale-down

4. **Pod Disruption Budgets**
   - API: minimum 2 replicas available
   - Dashboard: minimum 1 replica available
   - Ensures high availability during maintenance

### Services

- **slskr-api**: ClusterIP service for internal API access
- **slskr-dashboard**: ClusterIP service for dashboard
- **slskr-api-nodeport**: NodePort service for external access (port 30080)
- **Ingress**: HTTP/HTTPS routing with TLS termination

## Configuration

### Environment Variables

Edit `k8s/configmap.yaml` to customize:

```yaml
SLSKR_LOG_LEVEL: "info"           # Logging level
SLSKR_API_PORT: "8080"            # API port
SLSKR_METRICS_PORT: "9090"        # Metrics port
SLSKR_SESSION_MAX_IDLE: "300"     # Session timeout
SLSKR_TRANSFER_MAX_ACTIVE: "8"    # Max concurrent transfers
SLSKR_SEARCH_RESULT_LIMIT: "100"  # Search result limit
SLSKR_DATABASE_PATH: "/data/slskr.db"
SLSKR_WEBHOOK_TIMEOUT: "30"       # Webhook timeout seconds
SLSKR_WEBHOOK_MAX_RETRIES: "3"    # Webhook retry count
```

### Resource Requests/Limits

Adjust resource allocation in `k8s/deployment.yaml`:

```yaml
resources:
  requests:
    cpu: 250m        # Guaranteed CPU
    memory: 256Mi    # Guaranteed memory
  limits:
    cpu: 1000m       # Maximum CPU
    memory: 512Mi    # Maximum memory
```

## Monitoring

### Prometheus Integration

The deployment includes ServiceMonitor for automatic Prometheus scraping:

```bash
# Verify ServiceMonitor is created
kubectl get servicemonitor -n slskr

# Check metrics scrape configuration
kubectl logs -n prometheus prometheus-0 | grep slskr
```

### Available Metrics

All metrics are available at `/api/metrics`:

- HTTP request latency (p50, p95, p99)
- Active searches and transfers
- Message throughput
- User connections
- Database operations
- Webhook success/failure rates

### Grafana Dashboards

Import included Grafana dashboards:

1. Dashboard ID: slskr-api-overview
2. Dashboard ID: slskr-transfers
3. Dashboard ID: slskr-webhooks

## Scaling

### Manual Scaling

```bash
# Scale API to 5 replicas
kubectl scale deployment slskr-api -n slskr --replicas=5

# Scale dashboard to 3 replicas
kubectl scale deployment slskr-dashboard-standalone -n slskr --replicas=3
```

### Auto Scaling

The HPA (HorizontalPodAutoscaler) automatically scales based on metrics:

```bash
# View HPA status
kubectl get hpa -n slskr
kubectl describe hpa slskr-api-hpa -n slskr

# View scaling events
kubectl get events -n slskr --sort-by='.lastTimestamp'
```

## High Availability

### Multi-Region Deployment

For multi-region deployments:

1. Deploy separate instances per region
2. Use global load balancing (AWS Route53, GCP Cloud Load Balancer)
3. Implement cross-region replication for persistent data
4. Configure webhook delivery for cross-region sync

### Disaster Recovery

```bash
# Backup persistent data
kubectl exec -n slskr -it $(kubectl get pod -n slskr -l app=slskr-api -o name | head -1) -- \
  cp /data/slskr.db /tmp/backup.db
kubectl cp slskr/$(pod-name):/tmp/backup.db ./backup.db

# Restore from backup
kubectl cp ./backup.db slskr/$(pod-name):/data/slskr.db
```

## Security

### Network Policies

Create network policies to restrict traffic:

```bash
# Allow ingress only from nginx-ingress namespace
kubectl apply -f - << 'EOF'
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: slskr-ingress-only
  namespace: slskr
spec:
  podSelector:
    matchLabels:
      app: slskr-api
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
EOF
```

### RBAC Security

The deployment uses minimal RBAC permissions:
- Read pods and configmaps
- Read secrets (for API keys)
- No create/update/delete permissions

### Pod Security Standards

The deployment enforces:
- Non-root user (uid 1000)
- Read-only root filesystem where possible
- No privilege escalation
- Dropped all Linux capabilities

## Troubleshooting

### Pods not starting

```bash
# Check pod status
kubectl describe pod -n slskr <pod-name>

# Check container logs
kubectl logs -n slskr <pod-name> <container-name>

# Check events
kubectl get events -n slskr
```

### High memory usage

```bash
# Check memory consumption
kubectl top pod -n slskr

# Adjust memory limits in deployment.yaml
# Restart pods to apply changes
kubectl rollout restart deployment/slskr-api -n slskr
```

### Webhook delivery failures

```bash
# Check webhook logs
kubectl logs -n slskr -l app=slskr-api -c slskr-api | grep webhook

# Verify webhook configuration
curl http://localhost:8080/api/admin/webhooks
```

## Updates and Rollbacks

### Rolling Update

```bash
# Update image
kubectl set image deployment/slskr-api \
  slskr-api=your-registry/slskr:v1.1 \
  -n slskr

# Monitor rollout
kubectl rollout status deployment/slskr-api -n slskr

# Check rollout history
kubectl rollout history deployment/slskr-api -n slskr
```

### Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/slskr-api -n slskr

# Rollback to specific revision
kubectl rollout undo deployment/slskr-api --to-revision=2 -n slskr
```

## Production Checklist

- [ ] Update image URL in deployment.yaml
- [ ] Configure TLS certificates with cert-manager
- [ ] Set up monitoring with Prometheus
- [ ] Configure backup strategy for /data volume
- [ ] Set up logging aggregation (ELK, Datadog, etc.)
- [ ] Configure ingress hostname (slskr.example.com)
- [ ] Set resource limits appropriate for your cluster
- [ ] Enable network policies
- [ ] Configure RBAC for application access
- [ ] Set up alerting rules for critical metrics
- [ ] Document runbooks for common issues
- [ ] Test disaster recovery procedures

## Support

For issues or questions:
- Check logs: `kubectl logs -n slskr <pod>`
- Check events: `kubectl get events -n slskr`
- Review API health: `curl http://localhost:8080/api/health`
- Check Prometheus metrics: `http://prometheus:9090`
- Check Grafana dashboards: `http://grafana:3000`
