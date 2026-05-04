# slskR Migration Guide: v1.0.1 → v1.1.0

## Overview

This guide provides step-by-step instructions for upgrading slskR from v1.0.1 to v1.1.0. The upgrade introduces major features (PostgreSQL, Redis caching, OAuth2, advanced monitoring) while maintaining backward compatibility with v1.0.1.

**Key Changes:**
- Optional PostgreSQL database (SQLite still supported)
- Redis caching layer (improves performance 4-5x)
- OAuth2/OIDC authentication support
- Distributed tracing (Jaeger integration)
- WebSocket message batching
- New monitoring dashboards

**Upgrade Time:** 30-60 minutes (single instance)

---

## 1. Pre-Upgrade Checklist

Before starting the migration:

- [ ] Backup current database: `sqlite3 /var/lib/slskr/slskr.db ".backup backup.db"`
- [ ] Backup configuration files: `cp -r /etc/slskr /etc/slskr.backup`
- [ ] Verify disk space: At least 2x database size free
- [ ] Review changelog for breaking changes
- [ ] Schedule maintenance window (30 minutes downtime)
- [ ] Notify users of upcoming maintenance
- [ ] Test migration in staging environment first

---

## 2. Single-Instance Migration (SQLite → SQLite + Redis)

### 2.1 Download v1.1.0 Binary

```bash
# Download release
wget https://github.com/slskr/slskr/releases/download/v1.1.0/slskr-linux-x86_64.tar.gz

# Extract
tar -xzf slskr-linux-x86_64.tar.gz -C /usr/local/bin/
chmod +x /usr/local/bin/slskr

# Verify version
slskr --version
# slskr 1.1.0 (build: 2026-09-01, commit: abc123)
```

### 2.2 Start Redis (Docker)

```bash
# Pull Redis image
docker pull redis:7-alpine

# Start Redis container
docker run -d \
  --name slskr-redis \
  --restart unless-stopped \
  -p 6379:6379 \
  -v redis_data:/data \
  redis:7-alpine redis-server --appendonly yes

# Verify Redis is running
redis-cli ping
# PONG
```

**Or Install Redis Locally:**
```bash
# Ubuntu/Debian
sudo apt-get install redis-server

# Start and enable
sudo systemctl start redis-server
sudo systemctl enable redis-server

# Verify
redis-cli ping
# PONG
```

### 2.3 Update Configuration

**Old Config (v1.0.1):**
```yaml
# /etc/slskr/config.yaml
http_bind: "0.0.0.0:5030"
api_token: "token123"
database_path: "/var/lib/slskr/slskr.db"
```

**New Config (v1.1.0):**
```yaml
# /etc/slskr/config.yaml
http_bind: "0.0.0.0:5030"
api_token: "token123"

# Database selection (sqlite or postgresql)
database:
  type: "sqlite"
  path: "/var/lib/slskr/slskr.db"

# New: Redis caching
cache:
  enabled: true
  redis_url: "redis://127.0.0.1:6379"
  ttl_seconds:
    search_results: 300
    user_profiles: 600
    room_info: 60

# New: Monitoring & Tracing
monitoring:
  prometheus_enabled: true
  prometheus_port: 9090
  jaeger_enabled: false
  jaeger_endpoint: "http://127.0.0.1:14268/api/traces"

# New: Authentication providers
auth:
  tokens_enabled: true
  oauth2_enabled: false  # Can be enabled later
  mfa_enabled: false     # Can be enabled later
```

### 2.4 Migrate Systemd Service

**Old Service (v1.0.1):**
```ini
# /etc/systemd/system/slskr.service
[Service]
Type=simple
ExecStart=/usr/bin/slskr daemon
```

**Updated Service (v1.1.0):**
```ini
# /etc/systemd/system/slskr.service
[Unit]
Description=slskR P2P File Search Service
After=network.target redis.service

[Service]
Type=simple
User=slskr
Group=slskr
ExecStart=/usr/bin/slskr daemon --config /etc/slskr/config.yaml
Restart=on-failure
RestartSec=10s

# New: Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/slskr /var/log/slskr

# New: Resource limits
MemoryLimit=500M
CPUQuota=400%

[Install]
WantedBy=multi-user.target
```

### 2.5 Perform Migration

**Step 1: Stop v1.0.1 daemon**
```bash
sudo systemctl stop slskr
```

**Step 2: Backup database**
```bash
sudo sqlite3 /var/lib/slskr/slskr.db ".backup /var/lib/slskr/slskr.db.backup.$(date +%Y%m%d)"
```

**Step 3: Run migration script**
```bash
# The new version automatically handles schema migration
# No manual steps needed, but verify:
slskr migrate --database-path /var/lib/slskr/slskr.db

# Output:
# Checking schema version: v1.0.0
# Applying migration: add_cache_metadata_tables
# Applying migration: add_monitoring_tables
# Schema updated to: v1.1.0
# ✓ Migration complete
```

**Step 4: Test Redis connection**
```bash
# Update config to enable Redis
# This will be done by the migration script automatically

# Test connection
redis-cli -h 127.0.0.1 -p 6379 ping
# PONG
```

**Step 5: Start v1.1.0 daemon**
```bash
sudo systemctl daemon-reload
sudo systemctl start slskr

# Verify startup
sleep 5
curl http://127.0.0.1:5030/api/health | jq .

# Expected:
# {
#   "status": "healthy",
#   "version": "1.0.1",
#   "cache": "enabled"
# }
```

**Step 6: Verify cache is working**
```bash
# Make a search request
curl "http://127.0.0.1:5030/api/search?query=test"

# Make the same request again (should be cached)
curl "http://127.0.0.1:5030/api/search?query=test"

# Check Redis
redis-cli KEYS "search:*"
# 1) "search:test:100"

redis-cli TTL "search:test:100"
# (integer) 287  # Remaining TTL
```

---

## 3. PostgreSQL Migration (Optional, Recommended)

### 3.1 Why Upgrade to PostgreSQL?

| Feature | SQLite | PostgreSQL |
|---|---|---|
| Concurrent writes | 1 (exclusive lock) | Unlimited |
| Read replicas | No | Yes (streaming replication) |
| Horizontal sharding | No | Yes |
| Full-text search | Limited | Advanced (GIN indices) |
| Max instance size | 1 | 10+ instances |
| Max throughput | 8,500 req/sec | 50,000+ req/sec |

### 3.2 PostgreSQL Setup

**Option A: Install Locally**
```bash
# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib

# Start and enable
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database
sudo -u postgres psql << EOF
CREATE DATABASE slskr;
CREATE USER slskr WITH PASSWORD 'secure_password_123';
ALTER ROLE slskr SET client_encoding TO 'utf8';
ALTER ROLE slskr SET default_transaction_isolation TO 'read committed';
ALTER ROLE slskr SET default_transaction_deferrable TO on;
ALTER ROLE slskr SET timezone TO 'UTC';
GRANT ALL PRIVILEGES ON DATABASE slskr TO slskr;
EOF

# Verify
psql -U slskr -d slskr -h 127.0.0.1 -c "SELECT 1;"
```

**Option B: Use Docker**
```bash
# Start PostgreSQL container
docker run -d \
  --name slskr-postgres \
  --restart unless-stopped \
  -e POSTGRES_DB=slskr \
  -e POSTGRES_USER=slskr \
  -e POSTGRES_PASSWORD=secure_password_123 \
  -p 5432:5432 \
  -v postgres_data:/var/lib/postgresql/data \
  postgres:15-alpine

# Wait for startup
sleep 10

# Verify
psql -U slskr -h 127.0.0.1 -d slskr -c "SELECT 1;"
```

### 3.3 Data Migration (SQLite → PostgreSQL)

**Option A: Automated Migration Tool**
```bash
# v1.1.0 includes built-in migration tool
slskr migrate-db \
  --from sqlite:/var/lib/slskr/slskr.db \
  --to postgresql://slskr:password@127.0.0.1:5432/slskr

# Output:
# Connecting to SQLite: /var/lib/slskr/slskr.db
# Connecting to PostgreSQL: 127.0.0.1:5432
# Migrating table: transfers (50,000 rows) [████████████████████] 100%
# Migrating table: searches (100,000 rows) [████████████████████] 100%
# Migrating table: messages (500,000 rows) [████████████████████] 100%
# Migration complete: 650,000 rows migrated
# Verification: ✓ Data integrity confirmed
```

**Option B: Manual Migration**
```bash
# Export from SQLite
sqlite3 /var/lib/slskr/slskr.db << EOF
.mode csv
.output transfers.csv
SELECT * FROM transfers;
.output searches.csv
SELECT * FROM searches;
.output messages.csv
SELECT * FROM messages;
EOF

# Create schema in PostgreSQL
psql -U slskr -d slskr << EOF
-- Copy schema creation (auto-generated from slskr migrate-db)
EOF

# Import data
psql -U slskr -d slskr -c "COPY transfers FROM 'transfers.csv' CSV;"
psql -U slskr -d slskr -c "COPY searches FROM 'searches.csv' CSV;"
psql -U slskr -d slskr -c "COPY messages FROM 'messages.csv' CSV;"
```

### 3.4 Update Configuration

**config.yaml:**
```yaml
database:
  type: "postgresql"
  url: "postgresql://slskr:password@127.0.0.1:5432/slskr"
  max_connections: 20
  connection_timeout: 30
  
  # PostgreSQL-specific options
  ssl_mode: "prefer"  # or "require" for production
  statement_cache_size: 100
  prepared_statement_cache_size: 100
```

### 3.5 Verify Migration

```bash
# Connect to PostgreSQL
psql -U slskr -d slskr -h 127.0.0.1

# Check row counts
SELECT 'transfers' as table_name, COUNT(*) FROM transfers
UNION ALL
SELECT 'searches' as table_name, COUNT(*) FROM searches
UNION ALL
SELECT 'messages' as table_name, COUNT(*) FROM messages;

# Expected output:
# table_name  | count
# transfers   | 50000
# searches    | 100000
# messages    | 500000
```

---

## 4. Advanced Features Setup

### 4.1 Enable OAuth2 (GitHub Example)

**Step 1: Create GitHub OAuth App**
1. Go to GitHub Settings → Developer settings → OAuth Apps
2. Click "New OAuth App"
3. Fill in:
   - Application name: `slskR`
   - Homepage URL: `https://slskr.example.com`
   - Authorization callback URL: `https://slskr.example.com/api/auth/oauth2/callback`
4. Copy Client ID and Client Secret

**Step 2: Update Configuration**
```yaml
auth:
  oauth2_enabled: true
  providers:
    github:
      client_id: "your_client_id"
      client_secret: "your_client_secret"  # Use env var in production
      scopes: ["user:email"]
```

**Step 3: Test OAuth2**
```bash
# Get authorization URL
curl "http://127.0.0.1:5030/api/auth/oauth2/authorize?provider=github"

# Expected: Redirect to GitHub login page
# After login, redirected back to callback with code
# Exchange code for token
curl -X POST "http://127.0.0.1:5030/api/auth/oauth2/callback?code=abc123&provider=github"
```

### 4.2 Enable Multi-Factor Authentication (MFA)

```bash
# Enable MFA requirement for all users
curl -X POST http://127.0.0.1:5030/api/admin/config \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"mfa_required": true}'

# User enrolls in MFA
curl -X POST http://127.0.0.1:5030/api/user/mfa/enroll \
  -H "Authorization: Bearer $USER_TOKEN" \
  -d '{"method": "totp"}'

# Expected response:
# {
#   "secret": "JBSWY3DPEBLW64TMMQ======",
#   "qr_code": "data:image/png;base64,...",
#   "backup_codes": ["code1", "code2", ...]
# }
```

### 4.3 Enable Distributed Tracing

**config.yaml:**
```yaml
monitoring:
  jaeger_enabled: true
  jaeger_endpoint: "http://127.0.0.1:14268/api/traces"
  jaeger_agent_host: "127.0.0.1"
  jaeger_agent_port: 6831
  sample_rate: 0.1  # Sample 10% of requests (adjust as needed)
```

**Start Jaeger:**
```bash
docker run -d \
  --name jaeger \
  --restart unless-stopped \
  -p 6831:6831/udp \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest

# Access Jaeger UI
# http://127.0.0.1:16686
```

---

## 5. Data Validation & Rollback

### 5.1 Post-Migration Validation

```bash
#!/bin/bash
# validate_migration.sh

echo "=== slskR v1.1.0 Migration Validation ==="

# Check service health
echo "1. Service health..."
HEALTH=$(curl -s http://127.0.0.1:5030/api/health)
if echo "$HEALTH" | jq -e '.status == "healthy"' > /dev/null; then
    echo "✓ Service healthy"
else
    echo "✗ Service unhealthy"
    exit 1
fi

# Check database connectivity
echo "2. Database connectivity..."
TRANSFERS=$(curl -s "http://127.0.0.1:5030/api/transfers" | jq '.[] | length')
echo "✓ Database responsive, $TRANSFERS transfers found"

# Check Redis connectivity
echo "3. Redis cache..."
CACHE_HIT=$(redis-cli INFO stats | grep hits)
echo "✓ Redis connected, $CACHE_HIT"

# Check API response format (should include cache info)
echo "4. API response format..."
RESPONSE=$(curl -s "http://127.0.0.1:5030/api/search?query=test" | jq '.cache_hit')
if [ "$RESPONSE" != "null" ]; then
    echo "✓ API response format updated"
else
    echo "⚠ Cache info missing (expected if cache disabled)"
fi

echo ""
echo "=== Validation Complete ==="
```

### 5.2 Rollback Procedure

If issues occur, rollback to v1.0.1:

```bash
#!/bin/bash
# rollback_to_v1.0.1.sh

echo "Rolling back to v1.0.1..."

# Stop current version
sudo systemctl stop slskr

# Restore database backup
sudo sqlite3 /var/lib/slskr/slskr.db.backup.20260901 ".recover" | sqlite3 /var/lib/slskr/slskr.db

# Restore old binary
sudo cp /usr/bin/slskr.bak /usr/bin/slskr

# Restore old config
sudo cp -r /etc/slskr.backup/* /etc/slskr/

# Start old version
sudo systemctl start slskr

# Verify
sleep 5
curl http://127.0.0.1:5030/api/health

echo "Rollback complete. Please investigate and try again."
```

---

## 6. Performance Tuning

### 6.1 Redis Cache Optimization

```bash
# Check Redis memory usage
redis-cli INFO memory

# Monitor cache hit rate
redis-cli INFO stats | grep -E "hits|misses"

# Optimize TTLs based on your workload
# Search results: 5 minutes
# User profiles: 10 minutes
# Room info: 1 minute
# Peer info: 5 minutes
```

### 6.2 PostgreSQL Optimization

```bash
# Create indices for faster queries
psql -U slskr -d slskr << EOF
CREATE INDEX idx_transfers_user_id ON transfers(user_id);
CREATE INDEX idx_transfers_status ON transfers(status);
CREATE INDEX idx_searches_query_time ON searches(query, created_at DESC);
CREATE INDEX idx_messages_room_time ON messages(room_id, created_at DESC);

-- Analyze query plans
ANALYZE;
EOF

# Check query performance
psql -U slskr -d slskr -c "EXPLAIN ANALYZE SELECT * FROM transfers WHERE user_id = 'user123';"
```

### 6.3 Monitor Performance

```bash
# Before and after benchmarking
ab -n 10000 -c 100 "http://127.0.0.1:5030/api/search?query=test"

# Expected improvements:
# - Latency: 8ms → 2ms (4x faster with caching)
# - Throughput: 8,500 req/sec → 12,000 req/sec (if PostgreSQL)
```

---

## 7. Multi-Instance Deployment (Scale-Out)

If upgrading to a scaled deployment:

### 7.1 Shared Database Setup

**PostgreSQL Primary-Replica:**
```bash
# On primary (write) server
sudo nano /etc/postgresql/15/main/postgresql.conf

# Add:
wal_level = replica
max_wal_senders = 5
max_replication_slots = 5

# On replica (read) server
sudo -u postgres psql -c "SELECT pg_basebackup(...)"
```

### 7.2 Shared Redis Setup

```bash
# Use Redis Cluster for HA
redis-server --port 7000 --cluster-enabled yes --cluster-config-file nodes.conf
redis-server --port 7001 --cluster-enabled yes --cluster-config-file nodes.conf
redis-server --port 7002 --cluster-enabled yes --cluster-config-file nodes.conf

# Create cluster
redis-cli --cluster create 127.0.0.1:7000 127.0.0.1:7001 127.0.0.1:7002
```

### 7.3 Load Balancer Configuration (Nginx)

```nginx
upstream slskr {
    least_conn;
    server instance1.internal:5030;
    server instance2.internal:5030;
    server instance3.internal:5030;
}

server {
    listen 443 ssl http2;
    server_name slskr.example.com;
    
    location /api/ {
        proxy_pass http://slskr;
        proxy_set_header X-Forwarded-For $remote_addr;
    }
}
```

---

## 8. Migration Timeline (Single Instance)

| Step | Time | Notes |
|---|---|---|
| **Pre-Migration** | 10 min | Backup, preparation |
| **Stop Service** | 2 min | Brief downtime starts |
| **Download v1.1.0** | 5 min | Binary download |
| **Start Redis** | 3 min | Cache infrastructure |
| **Run Migration** | 5 min | Schema update |
| **Verify Setup** | 5 min | Connectivity checks |
| **Start Service** | 2 min | Service starts, downtime ends |
| **Post-Validation** | 10 min | Run tests, verify cache |
| **Total Downtime** | ~15 min | Minimal impact |
| **Total Time** | ~45 min | Including validation |

---

## 9. Troubleshooting Common Issues

### Redis Connection Failed

```
Error: Unable to connect to Redis at 127.0.0.1:6379
```

**Solution:**
```bash
# Check if Redis is running
sudo systemctl status redis-server
# or
docker ps | grep redis

# Check firewall
sudo ufw allow 6379

# Verify configuration
grep redis_url /etc/slskr/config.yaml

# Test connection
redis-cli ping
# Expected: PONG
```

### PostgreSQL Migration Timeout

```
Error: Migration timeout after 300 seconds
```

**Solution:**
```bash
# Increase timeout
slskr migrate-db \
  --from sqlite:/var/lib/slskr/slskr.db \
  --to postgresql://... \
  --timeout 1800  # 30 minutes

# Or migrate in smaller batches
slskr migrate-db --batch-size 10000
```

### Cache Hit Rate Low (< 50%)

```
Warning: Cache hit rate is 30%, expected > 70%
```

**Solution:**
```bash
# Check TTL values (may be too short)
grep ttl_seconds /etc/slskr/config.yaml

# Increase TTL for high-traffic data
# search_results: 300 → 600
# user_profiles: 600 → 900

# Monitor cache effectiveness
redis-cli INFO stats | grep -E "hits|misses"
```

---

## 10. Post-Migration Monitoring

### 10.1 Key Metrics to Watch

```bash
# Monitor every 30 minutes for first day:
while true; do
    echo "=== $(date) ==="
    
    # API health
    curl -s http://127.0.0.1:5030/api/health | jq '.status'
    
    # Cache hit rate
    redis-cli INFO stats | grep hits
    
    # Database connections
    curl -s http://127.0.0.1:5030/api/metrics | grep db_connection
    
    # Memory usage
    ps aux | grep slskr | grep -v grep | awk '{print $6 " MB"}'
    
    sleep 30m
done
```

### 10.2 Monitoring Checklist

- [ ] Service is stable (uptime > 4 hours)
- [ ] Cache hit rate > 70%
- [ ] API latency < 10ms (p95)
- [ ] Error rate < 0.1%
- [ ] Database size stable (not growing unexpectedly)
- [ ] Memory usage stable (< 300MB RSS)
- [ ] No errors in logs

---

## 11. Next Steps

After successful migration:

1. **Enable Monitoring** (MONITORING_OBSERVABILITY.md)
   - Setup Prometheus + Grafana
   - Configure alerting rules
   - Create dashboards

2. **Optimize Performance** (PERFORMANCE_BENCHMARK.md)
   - Run load tests
   - Tune cache TTLs
   - Optimize database indices

3. **Plan Multi-Instance Deployment** (if needed)
   - Setup load balancer
   - Deploy multiple instances
   - Configure shared database + cache

4. **Plan v1.1.x Features**
   - OAuth2/OIDC (if not enabled)
   - Distributed tracing
   - WebSocket optimizations

---

## Support & Rollback

**Still have issues?**
- Check migration logs: `/var/log/slskr/migration.log`
- Rollback using: `./rollback_to_v1.0.1.sh`
- File GitHub issue: https://github.com/slskr/slskr/issues

**Successful migration?**
- ✅ Update your documentation
- ✅ Share feedback with community
- ✅ Schedule post-migration review

---

## Summary

**Before Migration:**
- Single instance, SQLite, no caching
- 8,500 req/sec capacity
- p95 latency: 9ms
- No distributed tracing

**After Migration:**
- Single instance, SQLite + Redis cache, optional PostgreSQL
- 12,000+ req/sec capacity (with caching)
- p95 latency: 2-3ms (with cache hits)
- Distributed tracing available
- Monitoring infrastructure ready
- PostgreSQL path available for scaling

**Congratulations! Your slskR deployment is now running v1.1.0.**
