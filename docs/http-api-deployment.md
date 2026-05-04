# soulseekR HTTP API - Deployment & Troubleshooting Guide

## Quick Start

### 1. Build soulseekR

```bash
cd /path/to/soulseekR
cargo build --release
```

### 2. Configure HTTP API

Create or update `slskr.config.toml`:

```toml
# Basic Configuration
username = "your_username"
password = "your_password"
server_address = "slsk.example.com:2242"

# HTTP API Configuration
http_api_host = "0.0.0.0"           # Listen on all interfaces
http_api_port = 8080                # HTTP API port
http_api_bearer_token = "change-me" # Bearer token for authentication

# Optional: Shared directories
shared_directories = ["/music", "/downloads"]

# Optional: Transfer settings
transfer_max_active = 5
```

### 3. Start the Server

```bash
./target/release/slskr
```

### 4. Test the API

```bash
# Health check (no auth needed)
curl http://localhost:8080/api/health

# Get stats (requires auth)
curl -H "Authorization: Bearer change-me" \
     http://localhost:8080/api/stats
```

## Configuration Reference

### HTTP API Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `http_api_host` | string | `127.0.0.1` | IP address to bind to |
| `http_api_port` | integer | `8080` | Port number |
| `http_api_bearer_token` | string | (required) | Authentication token |
| `http_api_max_connections` | integer | `1000` | Max concurrent connections |
| `http_api_request_timeout_seconds` | integer | `30` | Request timeout |

### Security Options

```toml
# CORS configuration (optional)
http_api_cors_origins = ["http://localhost:3000", "https://example.com"]

# Rate limiting (optional, not currently implemented)
# http_api_rate_limit_requests = 1000
# http_api_rate_limit_window_seconds = 60

# TLS configuration (use reverse proxy instead)
# http_api_tls_cert_path = "/path/to/cert.pem"
# http_api_tls_key_path = "/path/to/key.pem"
```

## Deployment Scenarios

### Scenario 1: Local Development

**Goal**: Run soulseekR with API on localhost for testing

```toml
http_api_host = "127.0.0.1"
http_api_port = 8080
http_api_bearer_token = "dev-token"
```

**Access**: 
```bash
curl http://localhost:8080/api/health
```

### Scenario 2: Docker Deployment

**Dockerfile:**

```dockerfile
FROM rust:latest as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /build/target/release/slskr /usr/local/bin/
COPY --from=builder /build/slskr.config.toml /etc/soulseekr/
EXPOSE 8080
CMD ["slskr"]
```

**Run:**
```bash
docker build -t soulseekr .
docker run -p 8080:8080 \
           -e SLSKR_BEARER_TOKEN="my-token" \
           soulseekr
```

### Scenario 3: Production with Nginx

**Nginx Configuration:**

```nginx
upstream soulseekr {
    server localhost:8080;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location /api/ {
        proxy_pass http://soulseekr;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # CORS headers
        add_header 'Access-Control-Allow-Origin' '*' always;
        add_header 'Access-Control-Allow-Methods' 'GET,POST,PUT,DELETE,OPTIONS' always;
        
        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }
}
```

**Start soulseekR:**
```bash
./target/release/slskr --config /etc/soulseekr/slskr.config.toml
```

### Scenario 4: Kubernetes Deployment

**ConfigMap:**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: soulseekr-config
data:
  slskr.config.toml: |
    http_api_host = "0.0.0.0"
    http_api_port = 8080
    http_api_bearer_token = "kube-secret-token"
```

**Deployment:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: soulseekr
spec:
  replicas: 1
  selector:
    matchLabels:
      app: soulseekr
  template:
    metadata:
      labels:
        app: soulseekr
    spec:
      containers:
      - name: soulseekr
        image: soulseekr:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: config
          mountPath: /etc/soulseekr
        livenessProbe:
          httpGet:
            path: /api/health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /api/health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: soulseekr-config
---
apiVersion: v1
kind: Service
metadata:
  name: soulseekr-service
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 8080
  selector:
    app: soulseekr
```

## Troubleshooting

### Issue 1: Connection Refused

**Error:**
```
error: Connection refused (os error 111)
```

**Causes & Solutions:**

1. **Server not running**
   ```bash
   # Check if running
   ps aux | grep slskr
   
   # Start server
   ./target/release/slskr
   ```

2. **Wrong port**
   ```bash
   # Check configured port
   grep http_api_port slskr.config.toml
   
   # Test correct port
   curl http://localhost:8080/api/health
   ```

3. **Firewall blocking**
   ```bash
   # Check firewall rules
   sudo ufw status
   
   # Allow port
   sudo ufw allow 8080
   ```

### Issue 2: Unauthorized (401)

**Error:**
```json
{
  "error": "Unauthorized",
  "details": "Invalid or missing bearer token"
}
```

**Solutions:**

1. **Missing token**
   ```bash
   # Add Authorization header
   curl -H "Authorization: Bearer YOUR-TOKEN" \
        http://localhost:8080/api/stats
   ```

2. **Wrong token**
   ```bash
   # Verify token in config
   grep http_api_bearer_token slskr.config.toml
   
   # Update if needed
   sed -i 's/http_api_bearer_token = .*/http_api_bearer_token = "new-token"/' slskr.config.toml
   ```

3. **Token with spaces**
   ```bash
   # Ensure no spaces in token
   # ❌ Bad: "Bearer change me"
   # ✅ Good: "Bearer change-me"
   ```

### Issue 3: Forbidden (403)

**Error:**
```json
{
  "error": "Forbidden",
  "details": "CSRF origin validation failed"
}
```

**Solutions:**

1. **Missing Origin header (POST requests)**
   ```bash
   # Add Origin header
   curl -X POST \
        -H "Authorization: Bearer token" \
        -H "Origin: http://localhost:8080" \
        -H "Content-Type: application/json" \
        -d '{}' \
        http://localhost:8080/api/searches
   ```

2. **Wrong origin**
   ```bash
   # Origin must match server address
   # If server is 0.0.0.0:8080, use http://localhost:8080
   curl -H "Origin: http://localhost:8080" ...
   ```

### Issue 4: Timeout

**Error:**
```
error: timeout connecting to server
```

**Solutions:**

1. **Server overloaded**
   ```bash
   # Check system resources
   top
   
   # Check network connections
   netstat -an | grep 8080 | wc -l
   ```

2. **Network latency**
   ```bash
   # Test with longer timeout
   curl --max-time 60 http://localhost:8080/api/health
   ```

3. **Reverse proxy misconfigured**
   ```nginx
   # Ensure proxy timeouts are sufficient
   proxy_connect_timeout 30s;
   proxy_read_timeout 60s;
   ```

### Issue 5: High Memory Usage

**Symptoms:**
- Memory grows over time
- OOM killer triggered

**Solutions:**

1. **Check for memory leaks**
   ```bash
   # Monitor memory usage
   watch -n 1 'ps aux | grep slskr'
   
   # Profile with valgrind
   valgrind --leak-check=full ./target/release/slskr
   ```

2. **Limit event history**
   ```toml
   # In code, EVENT_HISTORY_LIMIT = 500
   # Adjust if needed
   ```

3. **Reduce message queue**
   ```toml
   # Limit stored messages
   # Currently bounded by implementation
   ```

### Issue 6: Slow API Responses

**Symptoms:**
- Requests take >1 second
- Inconsistent latency

**Solutions:**

1. **Check system load**
   ```bash
   # View load average
   uptime
   
   # List running processes
   ps aux --sort=-%cpu | head -10
   ```

2. **Check network bottleneck**
   ```bash
   # Monitor network
   iftop
   
   # Check for dropped packets
   netstat -i
   ```

3. **Profile with flamegraph**
   ```bash
   cargo flamegraph --bin slskr
   # Open flamegraph.svg in browser
   ```

4. **Check lock contention**
   ```bash
   # Use perf to identify hot paths
   perf record ./target/release/slskr
   perf report
   ```

### Issue 7: CORS Errors in Browser

**Error:**
```
Access to XMLHttpRequest has been blocked by CORS policy
```

**Solutions:**

1. **Add CORS headers (if using reverse proxy)**
   ```nginx
   add_header 'Access-Control-Allow-Origin' '$http_origin' always;
   add_header 'Access-Control-Allow-Methods' 'GET,POST,PUT,DELETE,OPTIONS' always;
   add_header 'Access-Control-Allow-Headers' 'Authorization,Content-Type' always;
   ```

2. **Use same domain**
   ```javascript
   // Instead of http://api.example.com:8080
   // Use same domain as frontend
   fetch('/api/stats', {
     headers: {'Authorization': 'Bearer token'}
   })
   ```

3. **Add OPTIONS handler**
   ```bash
   # soulseekR should handle OPTIONS automatically
   # If not, check proxy configuration
   ```

## Monitoring & Logging

### Health Checks

**Recommended Check Interval**: Every 30 seconds

```bash
#!/bin/bash
while true; do
  response=$(curl -s -H "Authorization: Bearer TOKEN" \
                      http://localhost:8080/api/health)
  if [[ $response == *"ok"* ]]; then
    echo "✓ soulseekR healthy at $(date)"
  else
    echo "✗ soulseekR unhealthy: $response"
    # Send alert
  fi
  sleep 30
done
```

### Metrics to Monitor

1. **Response Time**: Target <500ms (p95)
2. **Error Rate**: Target <0.1%
3. **Active Connections**: Watch for leaks
4. **CPU Usage**: Should be <50% idle
5. **Memory Usage**: Should be stable
6. **Request Volume**: Expected baseline

### Logging Configuration

Enable debug logging:

```bash
RUST_LOG=debug ./target/release/slskr
```

Redirect to file:

```bash
./target/release/slskr >> slskr.log 2>&1 &
```

Monitor logs:

```bash
tail -f slskr.log | grep -E "ERROR|WARN"
```

## Security Hardening

### 1. Use HTTPS

```bash
# Generate self-signed cert for testing
openssl req -x509 -newkey rsa:4096 \
           -keyout key.pem -out cert.pem \
           -days 365 -nodes

# Use reverse proxy with TLS
# See Nginx example above
```

### 2. Rotate Bearer Token Regularly

```bash
# Generate new token
openssl rand -hex 32

# Update configuration
sed -i 's/http_api_bearer_token = .*/http_api_bearer_token = "NEW-TOKEN"/' slskr.config.toml

# Restart server
pkill slskr
./target/release/slskr
```

### 3. Network Isolation

```bash
# Firewall to specific IPs only
sudo ufw allow from 192.168.1.0/24 to any port 8080

# Or via iptables
sudo iptables -A INPUT -p tcp --dport 8080 -s 192.168.1.0/24 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8080 -j DROP
```

### 4. Rate Limiting (Nginx)

```nginx
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=100r/s;
limit_req zone=api_limit burst=200 nodelay;
```

### 5. API Key Rotation Checklist

- [ ] Generate new token
- [ ] Update config
- [ ] Test with new token
- [ ] Notify API clients
- [ ] Update documentation
- [ ] Schedule rotation (quarterly minimum)

## Performance Tuning

### Nginx Configuration

```nginx
# Increase worker processes
worker_processes auto;

# Connection optimization
keepalive_timeout 65;
tcp_nodelay on;

# Buffer sizes
proxy_buffer_size 128k;
proxy_buffers 4 256k;

# Compression
gzip on;
gzip_types application/json;
gzip_min_length 1000;
```

### System Tuning

```bash
# Increase file descriptors
ulimit -n 65536

# Optimize TCP settings
sysctl -w net.ipv4.tcp_max_syn_backlog=4096
sysctl -w net.core.somaxconn=4096

# Increase connection queue
sysctl -w net.ipv4.tcp_fin_timeout=30
```

### Tokio Runtime Tuning

Currently uses default Tokio configuration. For high-load scenarios:

```rust
// In main.rs (future enhancement)
let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get())
    .thread_name("soulseekr-worker")
    .build()
    .unwrap();
```

## Backup & Recovery

### Configuration Backup

```bash
# Backup config
cp slskr.config.toml slskr.config.toml.bak

# Restore config
cp slskr.config.toml.bak slskr.config.toml
```

### State Backup

soulseekR state is stored in:
- Transfer queue (in-memory, not persisted)
- Message store (in-memory)
- Browse cache (in-memory, cleaned up)

For persistence, implement:
```bash
# Periodic backup of active transfers
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/transfers > transfers_backup.json
```

## Upgrading

### Zero-Downtime Upgrade

```bash
# 1. Build new version
cargo build --release

# 2. Graceful shutdown (optional)
kill -TERM $(pgrep slskr)
# Wait for in-flight requests to complete

# 3. Backup current binary
cp target/release/slskr target/release/slskr.old

# 4. Replace binary
cp target/release/slskr-new target/release/slskr

# 5. Start new version
./target/release/slskr
```

### Version Compatibility

- **Backwards compatible**: Old clients work with new servers
- **New clients + old servers**: May fail on new endpoints
- **Recommendation**: Keep clients and server versions aligned

## Support & Debugging

### Get System Information

```bash
curl -H "Authorization: Bearer token" \
     http://localhost:8080/api/config
```

### Enable Debug Logging

```bash
RUST_LOG=trace ./target/release/slskr
```

### Generate Bug Report

```bash
# Collect diagnostics
{
  echo "=== System Info ==="
  uname -a
  
  echo "=== soulseekR Version ==="
  curl http://localhost:8080/api/version
  
  echo "=== Config ==="
  curl -H "Authorization: Bearer token" \
       http://localhost:8080/api/config
  
  echo "=== Stats ==="
  curl -H "Authorization: Bearer token" \
       http://localhost:8080/api/stats
  
  echo "=== Recent Logs ==="
  tail -50 slskr.log
} > bug_report.txt
```

## References

- [HTTP API Documentation](http-api.md)
- [Performance Analysis](performance-analysis.md)
- [Nginx Documentation](https://nginx.org/en/docs/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
