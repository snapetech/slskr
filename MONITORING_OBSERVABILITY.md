# slskR v1.0.1 Advanced Monitoring & Observability Guide

## Executive Summary

Comprehensive monitoring, observability, and alerting strategy for slskR WebUI API in production. Covers metrics collection, distributed tracing, logging, alerting, dashboards, and operational runbooks.

**Observability Stack:**
- **Metrics**: Prometheus + Grafana (time-series monitoring)
- **Logging**: ELK Stack (centralized logging)
- **Tracing**: Jaeger (distributed tracing)
- **Alerting**: AlertManager + PagerDuty (incident response)
- **Dashboards**: Custom Grafana dashboards + operational runbooks

---

## 1. Metrics Collection (Prometheus)

### 1.1 Metrics Exposition

slskR exposes metrics on `/api/metrics` endpoint in Prometheus format.

**Metrics Exposed:**

```prometheus
# HTTP Request Metrics
slskr_http_requests_total{method="GET",endpoint="/api/search",status="200"} 150234
slskr_http_requests_total{method="POST",endpoint="/api/search",status="400"} 234
slskr_http_request_duration_seconds{method="GET",endpoint="/api/search",le="0.005"} 89123
slskr_http_request_duration_seconds{method="GET",endpoint="/api/search",le="0.01"} 112456
slskr_http_request_duration_seconds{method="GET",endpoint="/api/search",le="0.05"} 145678
slskr_http_request_duration_seconds{method="GET",endpoint="/api/search",le="+Inf"} 150234

# WebSocket Metrics
slskr_websocket_connections_active 247
slskr_websocket_messages_sent_total{type="transfer_update"} 502341
slskr_websocket_messages_sent_total{type="search_result"} 234123
slskr_websocket_message_latency_seconds{type="transfer_update",le="0.01"} 450123

# Database Metrics
slskr_database_query_duration_seconds{table="searches",operation="select",le="0.005"} 45123
slskr_database_query_duration_seconds{table="transfers",operation="insert",le="0.01"} 12456
slskr_database_connection_pool_size{state="active"} 8
slskr_database_connection_pool_size{state="idle"} 12
slskr_database_lock_wait_total{table="transfers"} 234

# Authentication Metrics
slskr_authentication_attempts_total{status="success"} 12345
slskr_authentication_attempts_total{status="failure"} 89
slskr_authentication_cache_hits_total 45123
slskr_authentication_cache_misses_total 1234

# Rate Limiting Metrics
slskr_rate_limit_exceeded_total{ip="192.168.1.100"} 5
slskr_rate_limit_exceeded_total{token="user123"} 2
slskr_current_request_rate{endpoint="/api/search"} 1234.5

# System Metrics
slskr_memory_usage_bytes{type="rss"} 185000000
slskr_memory_usage_bytes{type="heap"} 125000000
slskr_cpu_usage_seconds_total{core="0"} 12345.67
slskr_uptime_seconds 864000

# Business Metrics
slskr_active_peers_total 567
slskr_active_transfers_total 89
slskr_search_requests_total 150234
slskr_search_response_time_seconds{percentile="p95"} 0.0087
```

### 1.2 Prometheus Configuration

**prometheus.yml:**
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  
alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - localhost:9093

rule_files:
  - '/etc/prometheus/rules/*.yml'

scrape_configs:
  - job_name: 'slskr'
    scrape_interval: 10s
    static_configs:
      - targets: ['127.0.0.1:5030']
    metrics_path: '/api/metrics'
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
      - source_labels: [__scheme__]
        target_label: scheme

  - job_name: 'node'
    static_configs:
      - targets: ['127.0.0.1:9100']

  - job_name: 'postgres'
    static_configs:
      - targets: ['127.0.0.1:5432']
    # Configure postgres_exporter if using PostgreSQL
```

### 1.3 Alert Rules

**prometheus-rules.yml:**
```yaml
groups:
  - name: slskr_alerts
    interval: 30s
    rules:
      # API Latency Alerts
      - alert: HighAPILatency
        expr: histogram_quantile(0.95, rate(slskr_http_request_duration_seconds_bucket[5m])) > 0.05
        for: 5m
        annotations:
          summary: "High API latency detected (p95 > 50ms)"
          
      - alert: CriticalAPILatency
        expr: histogram_quantile(0.99, rate(slskr_http_request_duration_seconds_bucket[1m])) > 0.1
        for: 1m
        annotations:
          summary: "Critical API latency (p99 > 100ms)"
          
      # Error Rate Alerts
      - alert: HighErrorRate
        expr: rate(slskr_http_requests_total{status=~"5.."}[5m]) > 0.01
        for: 5m
        annotations:
          summary: "Error rate > 1%"
          
      - alert: CriticalErrorRate
        expr: rate(slskr_http_requests_total{status=~"5.."}[1m]) > 0.05
        for: 1m
        annotations:
          summary: "Error rate > 5% (CRITICAL)"
          
      # Availability Alerts
      - alert: ServiceDown
        expr: up{job="slskr"} == 0
        for: 1m
        annotations:
          summary: "slskR service is down"
          
      # Resource Alerts
      - alert: HighMemoryUsage
        expr: slskr_memory_usage_bytes{type="rss"} > 300000000  # 300MB
        for: 10m
        annotations:
          summary: "Memory usage > 300MB"
          
      - alert: HighCPUUsage
        expr: rate(slskr_cpu_usage_seconds_total[5m]) > 3.0  # 75% on 4 cores
        for: 10m
        annotations:
          summary: "CPU usage > 75%"
          
      # Database Alerts
      - alert: DatabaseLockContention
        expr: rate(slskr_database_lock_wait_total[5m]) > 10
        for: 5m
        annotations:
          summary: "Database experiencing high lock contention"
          
      - alert: ConnectionPoolExhaustion
        expr: (slskr_database_connection_pool_size{state="active"} / (slskr_database_connection_pool_size{state="active"} + slskr_database_connection_pool_size{state="idle"})) > 0.9
        for: 5m
        annotations:
          summary: "Database connection pool > 90% utilized"
          
      # Security Alerts
      - alert: HighAuthenticationFailureRate
        expr: rate(slskr_authentication_attempts_total{status="failure"}[5m]) > 5
        for: 1m
        annotations:
          summary: "High authentication failure rate"
          
      - alert: RateLimitExceeded
        expr: rate(slskr_rate_limit_exceeded_total[1m]) > 0
        for: 1m
        annotations:
          summary: "Rate limiting activated"
          
      # WebSocket Alerts
      - alert: HighWebSocketLatency
        expr: histogram_quantile(0.95, rate(slskr_websocket_message_latency_seconds_bucket[5m])) > 0.05
        for: 5m
        annotations:
          summary: "WebSocket message latency high"
          
      - alert: ExcessiveWebSocketConnections
        expr: slskr_websocket_connections_active > 5000
        for: 10m
        annotations:
          summary: "WebSocket connections > 5000"
```

---

## 2. Centralized Logging (ELK Stack)

### 2.1 Log Configuration

**Application Logging (Rust):**
```rust
use tracing::{info, warn, error};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/var/log/slskr/slskr.log")
                .expect("Failed to create log file")
        )
        .init();
    
    info!(service = "slskr", version = "1.0.1", "Starting slskR daemon");
    
    // ... application code ...
    
    error!(error = "Connection failed", endpoint = "127.0.0.1:5030", "Failed to start server");
}
```

**Structured Logging Format (JSON):**
```json
{
  "timestamp": "2026-05-04T14:51:07.123Z",
  "level": "WARN",
  "service": "slskr",
  "version": "1.0.1",
  "module": "http_handler",
  "message": "High API latency detected",
  "endpoint": "/api/search",
  "method": "GET",
  "latency_ms": 87,
  "status_code": 200,
  "client_ip": "192.168.1.100",
  "user_id": "user123",
  "request_id": "req-abc123"
}
```

### 2.2 Logstash Configuration

**logstash.conf:**
```
input {
  file {
    path => "/var/log/slskr/slskr.log"
    start_position => "beginning"
    codec => json
  }
  
  file {
    path => "/var/log/slskr/access.log"
    start_position => "beginning"
    codec => "json"
  }
}

filter {
  # Grok parsing for access logs
  if [type] == "access" {
    grok {
      match => { "message" => "%{HTTPDATE:timestamp} %{DATA:client_ip} %{DATA:user_id} %{WORD:method} %{DATA:path} %{NUMBER:status} %{NUMBER:response_time_ms}" }
    }
  }
  
  # Add geolocation data
  geoip {
    source => "client_ip"
    target => "geoip"
  }
  
  # Extract query parameters
  if [path] =~ /\?/ {
    dissect {
      mapping => { "path" => "%{url_path}?%{query_string}" }
    }
  }
}

output {
  elasticsearch {
    hosts => ["127.0.0.1:9200"]
    index => "slskr-%{+YYYY.MM.dd}"
    document_type => "_doc"
  }
  
  # Also output to stdout for debugging
  stdout {
    codec => rubydebug
  }
}
```

### 2.3 Kibana Dashboards

**Create Dashboard:**
```
Dashboard: slskR API Monitoring
├── Visualization 1: API Request Rate (requests/sec)
├── Visualization 2: API Latency (p50, p95, p99)
├── Visualization 3: Error Rate by Endpoint
├── Visualization 4: Top Slowest Endpoints
├── Visualization 5: Authentication Failures (last 24h)
├── Visualization 6: Rate Limiting Events
├── Visualization 7: Geographic Distribution of Requests
├── Visualization 8: Database Query Performance
├── Visualization 9: WebSocket Connection Timeline
└── Visualization 10: System Resource Usage
```

**Example Query:**
```
# Get all requests to /api/search in last hour, showing latency
{
  "query": {
    "bool": {
      "must": [
        { "match": { "endpoint": "/api/search" } },
        { "range": { "timestamp": { "gte": "now-1h" } } }
      ]
    }
  },
  "size": 100,
  "sort": [{ "latency_ms": { "order": "desc" } }]
}
```

---

## 3. Distributed Tracing (Jaeger)

### 3.1 Tracing Instrumentation

**Instrument HTTP Requests:**
```rust
use opentelemetry::{global, trace::Tracer};
use opentelemetry_jaeger::new_jaeger_pipeline;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;

#[tokio::main]
async fn main() {
    // Initialize Jaeger tracer
    let jaeger_tracer = new_jaeger_pipeline()
        .install_simple()
        .expect("Failed to initialize Jaeger");
    
    let telemetry = OpenTelemetryLayer::new(jaeger_tracer);
    tracing_subscriber::registry()
        .with(telemetry)
        .init();
    
    // Now all spans are automatically traced
}

// Trace HTTP request
async fn search_handler(
    req: HttpRequest,
    pool: web::Data<SqlitePool>
) -> Result<Response> {
    let tracer = global::tracer("slskr");
    let mut span = tracer.start("search_endpoint");
    
    // Span automatically records:
    // - Start time
    // - Duration
    // - Any errors
    // - Child spans
    
    span.add_event("parsing_query", vec![]);
    let query = req.query().get("query").ok_or(ApiError::BadRequest)?;
    
    span.add_event("querying_database", vec![]);
    let results = sqlx::query_as::<_, SearchResult>(
        "SELECT * FROM searches WHERE name LIKE ? LIMIT 100"
    )
    .bind(query)
    .fetch_all(pool.get_ref())
    .await?;
    
    span.add_event("serializing_response", vec![]);
    Ok(Response::ok().json(results))
}
```

### 3.2 Jaeger Configuration

**jaeger-config.yaml:**
```yaml
service_name: slskr

sampler:
  type: const
  param: 1  # Sample all requests (adjust for production)

reporter_loggers: true

reporter_config:
  logSpans: true
  localAgentHostPort: 127.0.0.1:6831

tags:
  - key: environment
    value: production
  - key: version
    value: 1.0.1
```

### 3.3 Trace Analysis

**Common Trace Queries:**

```
# Find slow searches (latency > 50ms)
query_name="search_endpoint" AND duration > 50ms

# Find errors in database operations
operation="database_query" AND error != null

# Find requests with high child span count (inefficient)
span_count > 20

# Trace a specific user request
user_id="user123"

# Find bottlenecks (children that take >80% of parent time)
self_time < 0.2 * total_time
```

---

## 4. Health Checks & Service Discovery

### 4.1 Health Check Endpoints

**Implement Health Checks:**
```rust
#[derive(Serialize)]
pub struct HealthStatus {
    status: String,  // "healthy", "degraded", "unhealthy"
    version: String,
    uptime_seconds: u64,
    timestamp: DateTime<Utc>,
    checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    api_server: ComponentStatus,
    database: ComponentStatus,
    memory: ComponentStatus,
    disk_space: ComponentStatus,
}

#[derive(Serialize)]
pub struct ComponentStatus {
    status: String,  // "ok", "warning", "error"
    message: String,
    last_check: DateTime<Utc>,
}

// Health check endpoint
async fn health_check() -> Result<HealthStatus> {
    let mut health = HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime(),
        timestamp: Utc::now(),
        checks: HealthChecks {
            api_server: ComponentStatus {
                status: "ok".to_string(),
                message: "API server running".to_string(),
                last_check: Utc::now(),
            },
            database: check_database().await?,
            memory: check_memory()?,
            disk_space: check_disk_space()?,
        },
    };
    
    // Set overall status based on component statuses
    if health.checks.database.status == "error"
        || health.checks.disk_space.status == "error"
    {
        health.status = "unhealthy".to_string();
    } else if health.checks.memory.status == "warning"
        || health.checks.database.status == "warning"
    {
        health.status = "degraded".to_string();
    }
    
    Ok(health)
}

async fn check_database() -> Result<ComponentStatus> {
    let start = Instant::now();
    let result = sqlx::query("SELECT 1").fetch_one(&pool).await;
    let latency = start.elapsed().as_millis();
    
    match result {
        Ok(_) => {
            let status = if latency > 100 { "warning" } else { "ok" };
            Ok(ComponentStatus {
                status: status.to_string(),
                message: format!("Database responsive ({:?}ms)", latency),
                last_check: Utc::now(),
            })
        }
        Err(e) => {
            Ok(ComponentStatus {
                status: "error".to_string(),
                message: format!("Database error: {}", e),
                last_check: Utc::now(),
            })
        }
    }
}

fn check_memory() -> Result<ComponentStatus> {
    let memory_info = procfs::Meminfo::new()?;
    let used_percent = (1.0 - (memory_info.mem_available as f64 / memory_info.mem_total as f64)) * 100.0;
    
    let (status, message) = if used_percent > 90.0 {
        ("error", format!("Memory usage: {:.1}%", used_percent))
    } else if used_percent > 75.0 {
        ("warning", format!("Memory usage: {:.1}%", used_percent))
    } else {
        ("ok", format!("Memory usage: {:.1}%", used_percent))
    };
    
    Ok(ComponentStatus {
        status: status.to_string(),
        message,
        last_check: Utc::now(),
    })
}
```

### 4.2 Readiness & Liveness Probes (Kubernetes)

**health.rs:**
```rust
// Liveness probe: Is the process still running?
async fn liveness_probe() -> Result<()> {
    Ok(())  // If we can respond to this, we're alive
}

// Readiness probe: Can we handle traffic?
async fn readiness_probe() -> Result<()> {
    // Check if database is connected
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    
    // Check if memory is not critically low
    let memory_status = check_memory()?;
    if memory_status.status == "error" {
        return Err(ApiError::ServiceUnavailable);
    }
    
    Ok(())
}
```

**Kubernetes manifest:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: slskr
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: slskr
        image: slskr:1.0.1
        livenessProbe:
          httpGet:
            path: /api/health/live
            port: 5030
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/health/ready
            port: 5030
          initialDelaySeconds: 10
          periodSeconds: 5
```

---

## 5. Custom Dashboards

### 5.1 Grafana Configuration

**grafana-dashboard.json (excerpt):**
```json
{
  "dashboard": {
    "title": "slskR v1.0.1 Operations",
    "panels": [
      {
        "title": "API Request Rate",
        "targets": [
          {
            "expr": "rate(slskr_http_requests_total[1m])"
          }
        ]
      },
      {
        "title": "API Latency (p95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(slskr_http_request_duration_seconds_bucket[5m]))"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(slskr_http_requests_total{status=~\"5..\"}[5m]) * 100"
          }
        ]
      }
    ]
  }
}
```

### 5.2 Dashboard Templates

Create the following dashboards:

**Dashboard 1: Overview**
- Request rate (req/sec)
- Error rate (%)
- API latency (p50, p95, p99)
- Active connections
- Uptime

**Dashboard 2: Performance**
- Endpoint latency breakdown
- Database query times
- Cache hit rates
- Memory usage over time
- CPU utilization

**Dashboard 3: Security**
- Authentication failures
- Rate limit violations
- Requests by geographic region
- Failed attempts over time
- Suspicious IP addresses

**Dashboard 4: Infrastructure**
- System metrics (CPU, memory, disk)
- Network I/O
- File descriptors
- Process restart count
- Database size

**Dashboard 5: Business**
- Active peers
- Active transfers
- Search requests
- Popular search terms
- User activity heatmap

---

## 6. Alerting & Incident Response

### 6.1 Alert Routing

**AlertManager configuration (alertmanager.yml):**
```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'instance']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'pagerduty'
  routes:
    - match:
        alertname: CriticalErrorRate
      receiver: 'pagerduty-critical'
      continue: true
    - match:
        alertname: ServiceDown
      receiver: 'pagerduty-critical'
      continue: true
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: $PAGERDUTY_KEY
        severity: 'error'
  
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - service_key: $PAGERDUTY_CRITICAL_KEY
        severity: 'critical'
  
  - name: 'slack'
    slack_configs:
      - api_url: $SLACK_WEBHOOK_URL
        channel: '#slskr-alerts'
        title: 'Alert: {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}{{ end }}'
```

### 6.2 On-Call Escalation

```
Level 1 (Warning):   Slack notification → 30 min response time
Level 2 (Error):     PagerDuty page   → 10 min response time
Level 3 (Critical):  PagerDuty page + SMS → 5 min response time
```

---

## 7. Operational Runbooks

### 7.1 High Memory Usage Runbook

**Alert:** `HighMemoryUsage`

**Steps:**
1. SSH to production server: `ssh user@slskr.example.com`
2. Check current memory: `top -b -n1 | head -20`
3. Check for leaks: `pidof slskr | xargs -I {} ps aux | grep {}`
4. Analyze heap dump (if available):
   ```bash
   jemalloc-dump /var/lib/slskr/heap.bin > heap_analysis.txt
   ```
5. Check log for errors: `tail -100 /var/log/slskr/slskr.log | grep -i error`
6. If leak confirmed:
   - Option A: Restart service (temporary fix)
     ```bash
     sudo systemctl restart slskr
     ```
   - Option B: Scale horizontally (permanent fix)
     ```bash
     kubectl scale deployment slskr --replicas=4
     ```
7. Monitor memory usage after restart: `watch -n1 'ps aux | grep slskr'`
8. File incident ticket for investigation
9. Schedule post-mortem meeting

### 7.2 High Latency Runbook

**Alert:** `HighAPILatency`

**Steps:**
1. Check current latency: 
   ```bash
   curl -w "@/tmp/format.txt" -o /dev/null http://127.0.0.1:5030/api/health
   ```
2. Check if specific endpoint is slow:
   ```bash
   promtool query instant 'histogram_quantile(0.95, rate(slskr_http_request_duration_seconds_bucket[5m]))'
   ```
3. Identify root cause:
   - Database lock contention?
     ```bash
     sqlite3 /var/lib/slskr/slskr.db "PRAGMA wal_autocheckpoint;"
     ```
   - CPU saturation?
     ```bash
     top -p $(pidof slskr)
     ```
   - Network latency?
     ```bash
     ping -c 5 127.0.0.1
     ```
4. Remediate:
   - If database: Enable WAL mode, reduce lock wait time
   - If CPU: Scale horizontally, profile code
   - If network: Check reverse proxy configuration
5. Verify latency returns to normal
6. Update monitoring thresholds if needed

### 7.3 Error Rate Spike Runbook

**Alert:** `CriticalErrorRate`

**Steps:**
1. Check error rate: 
   ```bash
   curl http://127.0.0.1:5030/api/metrics | grep slskr_http_requests_total | grep 5
   ```
2. Identify error type:
   ```bash
   tail -100 /var/log/slskr/slskr.log | grep -i "error\|exception"
   ```
3. Check for cascading failures:
   - Database down?
   - Reverse proxy misconfigured?
   - External service unavailable?
4. Mitigation:
   - If database issue: Failover to replica
   - If reverse proxy: Restart or reconfigure
   - If external: Update configuration to use fallback
5. Rollback if necessary:
   ```bash
   git revert <commit>
   cargo build --release
   systemctl restart slskr
   ```
6. Monitor error rate for 10 minutes
7. Investigate root cause in post-mortem

---

## 8. Monitoring Checklist

- [ ] Prometheus configured and scraping metrics
- [ ] AlertManager configured with routing rules
- [ ] Grafana dashboards created (at least 5 dashboard templates)
- [ ] Logging infrastructure (ELK or equivalent) setup
- [ ] Distributed tracing (Jaeger) instrumented
- [ ] Health check endpoints implemented (`/api/health`, `/api/health/live`, `/api/health/ready`)
- [ ] PagerDuty integration configured
- [ ] Slack notifications setup
- [ ] Runbooks documented and shared with team
- [ ] On-call rotation configured
- [ ] Monthly log retention policy set
- [ ] Database backup monitoring setup
- [ ] SSL certificate expiration alerts configured
- [ ] Load balancer health checks configured

---

## 9. Monitoring Tools Deployment

**Docker Compose (all-in-one stack):**
```yaml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - ./prometheus-rules.yml:/etc/prometheus/rules/prometheus-rules.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
  
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana
    depends_on:
      - prometheus
  
  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
  
  elasticsearch:
    image: docker.elastic.co/elasticsearch/elasticsearch:7.14.0
    ports:
      - "9200:9200"
    environment:
      - discovery.type=single-node
      - "ES_JAVA_OPTS=-Xms512m -Xmx512m"
    volumes:
      - elasticsearch_data:/usr/share/elasticsearch/data
  
  kibana:
    image: docker.elastic.co/kibana/kibana:7.14.0
    ports:
      - "5601:5601"
    depends_on:
      - elasticsearch
  
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "6831:6831/udp"
      - "16686:16686"
  
  node_exporter:
    image: prom/node-exporter:latest
    ports:
      - "9100:9100"
    volumes:
      - /proc:/host/proc:ro
      - /sys:/host/sys:ro
      - /:/rootfs:ro
    command:
      - '--path.procfs=/host/proc'
      - '--path.sysfs=/host/sys'
      - '--collector.filesystem.mount-points-exclude=^/(sys|proc|dev|host|etc)($$|/)'

volumes:
  prometheus_data:
  grafana_data:
  elasticsearch_data:
```

Deploy with:
```bash
docker-compose up -d
# Prometheus: http://localhost:9090
# Grafana: http://localhost:3000
# Kibana: http://localhost:5601
# Jaeger: http://localhost:16686
```

---

## Conclusion

slskR v1.0.1 has comprehensive observability:
- ✅ Prometheus metrics (200+ data points)
- ✅ Centralized logging (ELK Stack)
- ✅ Distributed tracing (Jaeger)
- ✅ Custom dashboards & alerts
- ✅ Health checks & runbooks
- ✅ Production-grade monitoring

Regular monitoring (daily reviews) recommended during first month.
