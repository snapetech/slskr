# SoulseekR Monitoring and Observability Guide

Complete guide for monitoring the slskr API with Prometheus, Grafana, and Alertmanager.

## Overview

The slskr deployment includes comprehensive monitoring:

- **Prometheus**: Metrics collection and storage (30-day retention)
- **Grafana**: Dashboards for visualization
- **Alertmanager**: Alert routing and notification
- **ServiceMonitor**: Automatic service discovery
- **PrometheusRule**: Alert rules and recording rules

## Installation

### 1. Install Prometheus Stack

```bash
# Add Prometheus Helm repository
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# Install kube-prometheus-stack
helm install kube-prometheus-stack prometheus-community/kube-prometheus-stack \
  -n monitoring --create-namespace \
  -f k8s/prometheus-values.yaml
```

### 2. Verify Installation

```bash
# Check Prometheus pods
kubectl get pods -n monitoring

# Check ServiceMonitor discovery
kubectl get servicemonitor -n slskr

# Verify metrics scraping
kubectl logs -n monitoring -l app.kubernetes.io/name=prometheus -f
```

## Metrics Available

### HTTP Request Metrics

- `http_requests_total`: Total HTTP requests (counter)
- `http_request_duration_seconds`: Request latency in seconds (histogram)
  - Buckets: 0.001, 0.01, 0.1, 0.5, 1, 5, 10
- `http_request_size_bytes`: Request payload size (histogram)
- `http_response_size_bytes`: Response payload size (histogram)

### Search Metrics

- `searches_total`: Total searches created (counter)
- `searches_completed_total`: Completed searches (counter)
- `search_queue_size`: Current search queue size (gauge)
- `search_result_count`: Results per search (histogram)
- `search_duration_seconds`: Time to complete search (histogram)

### Transfer Metrics

- `transfers_total`: Total transfers (counter)
- `transfers_completed_total`: Completed transfers (counter)
- `transfers_failed_total`: Failed transfers (counter)
- `active_transfers`: Current active transfers (gauge)
- `transfer_bytes_total`: Bytes transferred (counter)
- `transfer_duration_seconds`: Transfer duration (histogram)

### Message Metrics

- `messages_sent_total`: Messages sent (counter)
- `messages_received_total`: Messages received (counter)
- `message_delivery_latency_seconds`: Delivery latency (histogram)

### User & Session Metrics

- `user_connections_total`: User connections (counter)
- `user_disconnections_total`: User disconnections (counter)
- `active_sessions`: Currently active sessions (gauge)
- `session_duration_seconds`: Session duration (histogram)

### Webhook Metrics

- `webhook_deliveries_total`: Total webhook deliveries (counter, labeled by status)
- `webhook_delivery_latency_seconds`: Delivery latency (histogram)
- `webhook_retries_total`: Retry attempts (counter)

### System Metrics

- `process_cpu_seconds_total`: CPU time used (counter)
- `process_resident_memory_bytes`: Memory usage (gauge)
- `process_open_fds`: Open file descriptors (gauge)
- `process_max_fds`: Maximum file descriptors (gauge)

## Accessing Dashboards

### Prometheus

```bash
# Port-forward to Prometheus
kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090

# Open browser to http://localhost:9090
```

Query examples:

```promql
# Request rate (requests per second)
rate(http_requests_total[5m])

# Error rate (5xx responses)
rate(http_requests_total{status=~"5.."}[5m])

# P95 latency
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# Active transfers
active_transfers

# Search completion rate
rate(searches_completed_total[5m])
```

### Grafana

```bash
# Port-forward to Grafana
kubectl port-forward -n monitoring svc/kube-prometheus-stack-grafana 3000:3000

# Open browser to http://localhost:3000
# Default credentials: admin / changeme
```

#### Pre-built Dashboards

**1. API Overview Dashboard**

Shows:
- Request rate (RPS)
- Error rate (5xx)
- P50, P95, P99 latencies
- Active connections
- Database connections
- System CPU and memory

**2. Search Performance Dashboard**

Shows:
- Searches per second
- Search queue size
- Average search duration
- Result count distribution
- Target distribution (global, user, room)

**3. Transfers Dashboard**

Shows:
- Active transfers
- Transfer throughput (bytes/sec)
- Transfer failures
- Average transfer duration
- Peer distribution

**4. Webhooks Dashboard**

Shows:
- Webhook delivery rate
- Success/failure rate
- Delivery latency (P50, P95, P99)
- Retry attempts
- Event type distribution

**5. System Health Dashboard**

Shows:
- Pod CPU/memory usage
- Network I/O
- Disk usage
- Pod restart count
- Pod termination count

## Alert Rules

### Critical Alerts (Severity: critical)

1. **High Error Rate**
   - Condition: 5xx error rate > 5% for 5 minutes
   - Action: Page on-call engineer

2. **Pod Restart Loop**
   - Condition: More than 0.1 restarts/min for 5 minutes
   - Action: Investigate pod logs and events

### Warning Alerts (Severity: warning)

1. **High API Latency**
   - Condition: P95 latency > 1 second for 5 minutes
   - Action: Check database performance and load

2. **High Memory Usage**
   - Condition: Memory > 90% of limit for 5 minutes
   - Action: Consider scaling or adjusting limits

3. **Webhook Delivery Failures**
   - Condition: Failure rate > 10% for 10 minutes
   - Action: Check webhook endpoint health

4. **Large Search Queue**
   - Condition: Queue size > 1000 for 5 minutes
   - Action: Monitor for capacity issues

### Info Alerts (Severity: info)

1. **Max Transfers Reached**
   - Condition: Active transfers at limit
   - Action: Monitor for transfer bottlenecks

## Setting Up Alert Notifications

### Email Notifications

Edit `k8s/alertmanager-config.yaml`:

```yaml
global:
  resolve_timeout: 5m
  smtp_smarthost: 'smtp.gmail.com:587'
  smtp_auth_username: 'your-email@gmail.com'
  smtp_auth_password: 'your-app-password'

route:
  receiver: 'email'
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h

receivers:
- name: 'email'
  email_configs:
  - to: 'oncall@example.com'
    from: 'alerts@example.com'
    headers:
      Subject: 'SoulseekR Alert: {{ .GroupLabels.alertname }}'
```

### Slack Notifications

```yaml
receivers:
- name: 'slack'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
    channel: '#alerts'
    title: 'Alert: {{ .GroupLabels.alertname }}'
    text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

### PagerDuty Integration

```yaml
receivers:
- name: 'pagerduty'
  pagerduty_configs:
  - service_key: 'YOUR_SERVICE_KEY'
    description: '{{ .GroupLabels.alertname }}'
```

## Custom Queries and Dashboards

### Example Queries

**Dashboard Health Score**

```promql
(1 - (rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])))
* 100
```

**Transfer Throughput**

```promql
rate(transfer_bytes_total[5m]) / 1024 / 1024  # MB/sec
```

**Search Completion Time (P99)**

```promql
histogram_quantile(0.99, rate(search_duration_seconds_bucket[5m]))
```

**User Churn (sessions ended per minute)**

```promql
rate(user_sessions_ended_total[1m])
```

**API Availability**

```promql
(
  count(up{job="slskr-api"} == 1) 
  / 
  count(up{job="slskr-api"})
) * 100
```

## Data Retention and Archival

### Prometheus Retention

Default: 30 days of data

To change:

```bash
kubectl set env deployment/prometheus -n monitoring \
  PROMETHEUS_RETENTION_SIZE=50GB \
  PROMETHEUS_RETENTION_TIME=30d
```

### Long-term Storage (Thanos)

For retention beyond 30 days, install Thanos:

```bash
helm install thanos prometheus-community/thanos \
  -n monitoring \
  -f k8s/thanos-values.yaml
```

Thanos provides:
- Object storage backend (S3, GCS, Azure)
- Multi-year data retention
- Downsampling of old data
- Query layer across multiple Prometheus instances

## Performance Tuning

### Scrape Interval Adjustment

Default: 30 seconds

For finer granularity (more storage):

```yaml
prometheus:
  prometheusSpec:
    scrapeInterval: 15s
    evaluationInterval: 15s
```

For coarser granularity (less storage):

```yaml
prometheus:
  prometheusSpec:
    scrapeInterval: 60s
    evaluationInterval: 60s
```

### Recording Rules

Recording rules (pre-computed queries) improve dashboard performance:

```promql
# Available as instant queries without computation
slskr:http_request_duration:p95
slskr:http_requests:rate1m
slskr:webhook_success_rate
```

## Troubleshooting

### Metrics not appearing

```bash
# Check if ServiceMonitor exists
kubectl get servicemonitor -n slskr

# Check if endpoints match
kubectl get endpoints -n slskr slskr-api

# Verify pod labels match selector
kubectl get pods -n slskr -o yaml | grep -A 10 labels
```

### High memory usage in Prometheus

```bash
# Check cardinality of metrics
curl http://localhost:9090/api/v1/metadata | jq length

# Check series count
curl http://localhost:9090/api/v1/query?query=count\(.*\)
```

### Alert rule not firing

```bash
# Check if rule is loaded
kubectl logs -n monitoring prometheus-0 | grep "rule_files"

# Verify query condition
# Test query in Prometheus UI
```

## Dashboards as Code

Define dashboards using Jsonnet:

```bash
# Install jsonnet
brew install jsonnet

# Generate dashboard JSON
jsonnet k8s/dashboard-generator.jsonnet > dashboard.json

# Import into Grafana
curl -X POST http://grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @dashboard.json
```

## Compliance and Auditing

### Metrics Audit Trail

All metric queries are logged:

```bash
kubectl logs -n monitoring -l app.kubernetes.io/name=grafana | grep "query"
```

### GDPR Compliance

For PII in logs/metrics:

1. Use metric relabeling to remove sensitive labels
2. Configure Prometheus to drop metrics with user data
3. Use separate storage for audit logs

```yaml
metric_relabel_configs:
- source_labels: [__name__]
  regex: '.*_pii.*'
  action: drop
```

## Cost Optimization

### Storage Optimization

- Reduce scrape interval (higher interval = less data)
- Use shorter retention period
- Drop unnecessary metrics
- Enable compression

### Query Optimization

- Use recording rules for frequently used queries
- Cache expensive queries in Grafana
- Use pagination for large result sets

## Further Reading

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Dashboards](https://grafana.com/grafana/dashboards/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/naming/)
- [Alerting Best Practices](https://grafana.com/blog/2020/02/25/how-to-write-better-alerting-rules/)
