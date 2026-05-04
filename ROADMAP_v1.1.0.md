# slskR v1.1.0 Planning & Roadmap

## Overview

slskR v1.1.0 (Q3 2026) focuses on **scalability**, **advanced features**, and **enterprise readiness**. Building on v1.0.1's production foundation with 298+ endpoints and comprehensive security/monitoring, v1.1.0 introduces horizontal scaling, advanced caching, and multi-database support.

**Release Timeline:**
- **Alpha (June 2026)**: Feature development, performance testing
- **Beta (July 2026)**: Stability testing, documentation
- **RC (August 2026)**: Final bug fixes, security hardening
- **GA (September 2026)**: Production release

**Target Metrics:**
- 50,000+ req/sec (10-instance cluster)
- p95 latency: < 10ms (sustained)
- 99.99% availability (scale-out with load balancing)
- PostgreSQL horizontal scaling support

---

## 1. Phase 11: Query Result Caching & Redis Integration

### 1.1 Caching Architecture

**Dual-Layer Caching:**
```
┌─────────────────────────────────────────┐
│         Client Request                  │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│ Layer 1: In-Memory Cache (LRU)           │ ← Fastest (us)
│ - Search results (5min TTL)              │
│ - User profiles (10min TTL)              │
│ - Room info (1min TTL)                   │
└──────────────────┬───────────────────────┘
                   │ (miss)
                   ▼
┌──────────────────────────────────────────┐
│ Layer 2: Redis (Distributed Cache)       │ ← Fast (ms)
│ - Shared across instances                │
│ - Automatic invalidation                 │
│ - TTL-based expiration                   │
└──────────────────┬───────────────────────┘
                   │ (miss)
                   ▼
┌──────────────────────────────────────────┐
│ Layer 3: Database (SQLite/PostgreSQL)    │ ← Slow (tens of ms)
│ - Persistent storage                     │
│ - Authoritative source                   │
└──────────────────────────────────────────┘
```

### 1.2 Implementation Plan

**Dependencies to Add:**
```toml
redis = "0.24"
tokio-redis = "0.2"
moka = "0.12"  # In-memory LRU cache
dashmap = "5.5"  # Concurrent HashMap
```

**Cache Manager:**
```rust
use moka::future::Cache;
use redis::aio::ConnectionManager;

pub struct CacheManager {
    local_cache: Cache<String, Vec<u8>>,  // In-memory
    redis_conn: ConnectionManager,         // Distributed
}

impl CacheManager {
    pub async fn get_or_fetch<F, T>(
        &self,
        key: &str,
        ttl_secs: u64,
        fetch_fn: F,
    ) -> Result<T>
    where
        F: Fn() -> BoxFuture<'static, Result<T>>,
        T: Serialize + for<'de> Deserialize<'de>,
    {
        // Try local cache first
        if let Some(cached) = self.local_cache.get(key) {
            return serde_json::from_slice(&cached);
        }
        
        // Try Redis
        if let Ok(cached) = self.redis_conn.get::<_, Vec<u8>>(key).await {
            self.local_cache.insert(key.to_string(), cached.clone()).await;
            return serde_json::from_slice(&cached);
        }
        
        // Fetch from origin
        let result = fetch_fn().await?;
        let serialized = serde_json::to_vec(&result)?;
        
        // Cache in both layers
        self.local_cache.insert(key.to_string(), serialized.clone()).await;
        self.redis_conn.setex(key, ttl_secs as usize, serialized).await?;
        
        Ok(result)
    }
}
```

**Search Result Caching:**
```rust
async fn search_with_cache(
    query: &str,
    limit: i32,
    cache: &CacheManager,
) -> Result<Vec<FileResult>> {
    let cache_key = format!("search:{}:{}", query, limit);
    
    cache.get_or_fetch(
        &cache_key,
        300,  // 5-minute TTL
        || {
            Box::pin(async {
                sqlx::query_as::<_, FileResult>(
                    "SELECT * FROM files WHERE name LIKE ? LIMIT ?"
                )
                .bind(format!("%{}%", query))
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
        }
    ).await
}
```

### 1.3 Expected Impact

| Metric | Current | With Cache | Improvement |
|---|---|---|---|
| Search Latency (p95) | 9ms | 2ms | 4.5x faster |
| Database Queries/sec | 8,500 | 4,000 | 50% reduction |
| Bandwidth | Baseline | -45% | 45% savings |
| CPU Usage | 35% | 20% | 43% reduction |

### 1.4 Testing Requirements

- [ ] Cache hit rate > 70% for typical workloads
- [ ] TTL expiration working correctly
- [ ] Cache invalidation on write operations
- [ ] Distributed cache consistency (Redis)
- [ ] Performance under high concurrency (1000+ reqs/sec)
- [ ] Memory limits enforced (LRU eviction)

---

## 2. Phase 12: PostgreSQL Migration & Horizontal Scaling

### 2.1 Database Layer Abstraction

**Current (v1.0.1):** SQLite only (file-based)
**Target (v1.1.0):** PostgreSQL (network-accessible, scalable)

**Database Trait:**
```rust
#[async_trait]
pub trait Database {
    async fn search(&self, query: &str) -> Result<Vec<FileResult>>;
    async fn insert_transfer(&self, transfer: &Transfer) -> Result<()>;
    async fn get_transfers(&self, user_id: &str) -> Result<Vec<Transfer>>;
    // ... more methods
}

pub struct PostgresDb {
    pool: PgPool,
}

pub struct SqliteDb {
    pool: SqlitePool,
}

// Both implement Database trait
#[async_trait]
impl Database for PostgresDb { ... }

#[async_trait]
impl Database for SqliteDb { ... }
```

### 2.2 Migration Path

**Phase 1 (v1.0.1 → v1.1.0-alpha):**
- SQLite remains default
- PostgreSQL driver added (optional via feature flag)
- Schema compatible between both

**Phase 2 (v1.1.0-beta):**
- PostgreSQL becomes default for new installations
- SQLite → PostgreSQL migration script provided
- Both databases tested in parallel

**Phase 3 (v1.1.0-RC):**
- Performance tuning for PostgreSQL (indices, connection pooling)
- Horizontal read replication support
- Load balancing setup documented

**Phase 4 (v1.1.0-GA):**
- PostgreSQL fully supported in production
- SQLite deprecated (deprecated, not removed)
- Migration guide published

### 2.3 Schema Changes

**New PostgreSQL-Specific Features:**
```sql
-- Partitioning for large tables (transfer_records)
CREATE TABLE transfer_records (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(256) NOT NULL,
    file_path VARCHAR(2048) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) PARTITION BY RANGE (YEAR(created_at));

-- Full-text search indices
CREATE INDEX idx_search_fts ON files USING GIN(to_tsvector('english', name));

-- Materialized views for aggregate queries
CREATE MATERIALIZED VIEW user_stats_mv AS
SELECT user_id, COUNT(*) as transfer_count, SUM(bytes_transferred) as total_bytes
FROM transfer_records
GROUP BY user_id;
```

### 2.4 Testing Plan

- [ ] Migrate sample data (100K+ records)
- [ ] Verify query compatibility
- [ ] Load test PostgreSQL cluster (50,000 req/sec)
- [ ] Failover testing (replica)
- [ ] Point-in-time recovery validation

---

## 3. Phase 13: Message Batching & WebSocket Optimization

### 3.1 Batching Strategy

**Current:** Send updates immediately (one message per event)
**Optimized:** Accumulate 10ms, send batch (10-100x fewer packets)

**Implementation:**
```rust
pub struct BatchedBroadcaster {
    batch_timeout: Duration,
    max_batch_size: usize,
    pending_messages: Arc<DashMap<String, Vec<Message>>>,
}

async fn batch_broadcast(&self, channel: &str, message: Message) {
    // Add to batch
    self.pending_messages
        .entry(channel.to_string())
        .or_insert_with(Vec::new)
        .push(message);
    
    // Flush if batch is full
    if self.pending_messages.get(channel).map(|v| v.len()) >= self.max_batch_size {
        self.flush_batch(channel).await;
    }
}

async fn flush_batch(&self, channel: &str) {
    if let Some((_, messages)) = self.pending_messages.remove(channel) {
        // Send all messages in one WebSocket frame
        let payload = BroadcastBatch {
            channel: channel.to_string(),
            messages,
            timestamp: Utc::now(),
        };
        self.broadcast_json(&payload).await;
    }
}
```

### 3.2 Expected Impact

| Metric | Current | Batched | Improvement |
|---|---|---|---|
| Network Packets/sec | 10,000 | 1,000 | 90% reduction |
| Bandwidth | Baseline | -20% | 20% savings |
| WebSocket Latency p95 | 12ms | 18ms | +6ms trade-off (acceptable) |
| Server CPU | Baseline | -15% | 15% reduction |

### 3.3 Implementation Timeline

- Week 1: Design batch protocol
- Week 2: Implement batcher service
- Week 3: Integrate with WebSocket handler
- Week 4: Testing & performance validation

---

## 4. Phase 14: Advanced Authentication (OAuth2/OIDC)

### 4.1 Multi-Auth Support

**Current:** Token-based only
**Target:** Token + OAuth2 + OIDC + Multi-Factor Authentication

**Auth Methods:**
```rust
pub enum AuthMethod {
    BearerToken(String),           // Current
    OAuth2Code(String),             // New
    OIDCIdToken(String),            // New
    SAML2Assertion(String),         // Optional
    MFATotp(String, String),        // TOTP code
}

#[async_trait]
pub trait AuthProvider {
    async fn verify(&self, method: &AuthMethod) -> Result<User>;
}

pub struct TokenAuthProvider {
    token_store: Arc<TokenStore>,
}

pub struct OAuth2Provider {
    client_id: String,
    client_secret: String,
    token_endpoint: String,
}

pub struct MFAProvider {
    totp: TOTP,
}
```

### 4.2 OAuth2 Integration Example

**GitHub OAuth:**
```rust
pub async fn oauth2_callback(
    code: &str,
    provider: &OAuth2Provider,
) -> Result<User> {
    // Exchange code for access token
    let token = provider.exchange_code(code).await?;
    
    // Get user info from OAuth provider
    let user_info = provider.get_user_info(&token).await?;
    
    // Create or update local user
    let user = User {
        id: format!("oauth2:{}", user_info.id),
        username: user_info.login,
        email: user_info.email,
        avatar_url: user_info.avatar_url,
        oauth_provider: Some("github"),
        oauth_id: Some(user_info.id),
    };
    
    // Persist to database
    insert_or_update_user(&user).await?;
    
    // Generate session
    let session = create_session(&user).await?;
    
    Ok(user)
}
```

### 4.3 Multi-Factor Authentication (MFA)

```rust
pub async fn enable_mfa(user_id: &str) -> Result<MFASetup> {
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, user_id.as_bytes())?;
    let secret = totp.get_secret_base32();
    let qr_code = totp.get_qr_base64()?;
    
    // Store secret in database (encrypted)
    store_mfa_secret(user_id, &secret).await?;
    
    Ok(MFASetup {
        secret,
        qr_code,
        backup_codes: generate_backup_codes(10),
    })
}

pub async fn verify_mfa(user_id: &str, totp_code: &str) -> Result<bool> {
    let secret = retrieve_mfa_secret(user_id).await?;
    let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret.as_bytes())?;
    
    // Verify with 1-minute grace window
    Ok(totp.check_current(totp_code)?)
}
```

### 4.4 Testing Plan

- [ ] OAuth2 authorization code flow
- [ ] Token refresh mechanism
- [ ] OIDC ID token validation
- [ ] MFA enrollment & verification
- [ ] Backup codes (9-use limit)
- [ ] Session management with MFA

---

## 5. Phase 15: Distributed Tracing (Jaeger) Integration

### 5.1 Instrumentation

**v1.0.1:** Basic logging
**v1.1.0:** Full distributed tracing with parent-child spans

```rust
use opentelemetry::trace::{Tracer, Span};
use opentelemetry_jaeger::new_jaeger_pipeline;

#[tokio::main]
async fn main() {
    let tracer = new_jaeger_pipeline()
        .install_simple()
        .expect("Failed to initialize Jaeger");
    
    let app = build_app(tracer.clone());
    app.run().await;
}

async fn search_handler(
    tracer: &Tracer,
    query: &str,
) -> Result<Vec<FileResult>> {
    let mut span = tracer.start("search_endpoint");
    span.set_attribute("query", query);
    span.set_attribute("timestamp", Utc::now().to_string());
    
    span.add_event("parsing_input", vec![]);
    let validated_query = validate_query(query)?;
    
    span.add_event("querying_database", vec![]);
    let results = {
        let mut db_span = tracer.start_span("database_query");
        db_span.set_attribute("query_type", "search");
        
        database_query(&validated_query).await?
    };
    
    span.add_event("building_response", vec![]);
    Ok(results)
}
```

### 5.2 Trace Analysis Queries

```
# Find slow searches
query_endpoint AND duration > 50ms

# Find errors in chain
status="error" AND operation="database_query"

# Find inefficient requests (many child spans)
parent_span AND span_count > 50

# Trace a specific user's requests
user_id="user123"

# Find bottlenecks
child_span_duration > 0.8 * parent_span_duration
```

### 5.3 Expected Benefits

- Identify performance bottlenecks across microservices
- Debug complex request flows
- Visualize distributed request timing
- Correlate errors across services

---

## 6. Phase 16: Multi-Region Deployment & CDN

### 6.1 Architecture

```
┌─────────────────────────────────────────────────────────┐
│                Global CDN (CloudFlare)                  │
│  - Static assets caching                                │
│  - DDoS protection                                       │
│  - Geo-routing                                           │
└────────────────┬─────────────────────────────────────────┘
                 │
     ┌───────────┼───────────┐
     │           │           │
     ▼           ▼           ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│US Region │ │EU Region │ │AS Region │
│(N. Virgin)│(Frankfurt)│(Tokyo)    │
│          │ │          │ │          │
│ 3x slskr │ │ 3x slskr │ │ 3x slskr │
│ 1x PgSQL │ │ 1x PgSQL │ │ 1x PgSQL │
│ 1x Redis │ │ 1x Redis │ │ 1x Redis │
└──────────┘ └──────────┘ └──────────┘
     │           │           │
     └───────────┼───────────┘
                 │
         ┌───────▼────────┐
         │ Central MySQL  │
         │ (Read Replicas)│
         └────────────────┘
```

### 6.2 Data Synchronization

**Replication Strategy:**
- Write-primary in US region
- Read replicas in all regions
- Asynchronous replication (eventual consistency)
- Conflict resolution via last-write-wins

**Implementation:**
```rust
pub struct MultiRegionManager {
    primary_region: String,
    replica_regions: Vec<String>,
    databases: HashMap<String, Box<dyn Database>>,
}

impl MultiRegionManager {
    pub async fn write(&self, op: WriteOperation) -> Result<()> {
        // Write to primary
        self.databases[&self.primary_region].execute(&op).await?;
        
        // Async replicate to other regions
        for region in &self.replica_regions {
            let db = &self.databases[region];
            tokio::spawn(async move {
                if let Err(e) = db.execute(&op).await {
                    error!("Replication failed to {}: {}", region, e);
                }
            });
        }
        
        Ok(())
    }
    
    pub async fn read(&self, query: ReadQuery) -> Result<Vec<Row>> {
        // Read from local region if available
        let region = self.get_local_region();
        self.databases[region].query(&query).await
    }
}
```

### 6.3 Testing Plan

- [ ] Data consistency across regions (< 5 second lag)
- [ ] Failover when primary unavailable
- [ ] Conflict resolution (duplicate writes)
- [ ] Load distribution (geo-routing)

---

## 7. Phase 17: gRPC Protocol (Alternative to REST)

### 7.1 Dual Protocol Support

**Current:** REST (HTTP/1.1, JSON)
**Future:** REST + gRPC (HTTP/2, Protocol Buffers)

**Benefits:**
- 70% smaller payload size (binary protocol)
- 30% lower latency (multiplexing, header compression)
- Better for mobile clients (less bandwidth)
- Streaming support (server-sent updates)

**Proto Definition:**
```proto
syntax = "proto3";
package slskr;

message SearchRequest {
    string query = 1;
    int32 limit = 2;
}

message FileResult {
    string path = 1;
    int64 size = 2;
    string hash = 3;
}

message SearchResponse {
    repeated FileResult results = 1;
    int32 total = 2;
}

service SearchService {
    rpc Search(SearchRequest) returns (SearchResponse) {}
    rpc StreamSearchResults(SearchRequest) returns (stream FileResult) {}
}
```

**Implementation:**
```rust
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default)]
pub struct SearchServiceImpl;

#[tonic::async_trait]
impl SearchService for SearchServiceImpl {
    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();
        
        let results = database::search(&req.query, req.limit as i32)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let response = SearchResponse {
            results: results.into_iter().map(|f| FileResult {
                path: f.path,
                size: f.size,
                hash: f.hash,
            }).collect(),
            total: results.len() as i32,
        };
        
        Ok(Response::new(response))
    }
}
```

### 7.2 Migration Strategy

- v1.1.0: gRPC support (optional, parallel to REST)
- v1.2.0: gRPC as primary, REST deprecated
- v1.3.0: REST removed

### 7.3 Performance Comparison

| Metric | REST/JSON | gRPC/Protobuf | Improvement |
|---|---|---|---|
| Payload size | 2.5KB | 750B | 70% reduction |
| Latency | 8ms | 5.5ms | 31% faster |
| Bandwidth | Baseline | -65% | 65% savings |
| Connections | 1000 | 5000+ | 5x more concurrent |

---

## 8. Phase 18: Database Sharding (Optional v1.2.0)

### 8.1 Sharding Strategy

**Shard Key:** `user_id` (consistent hash)

```rust
pub struct ShardingManager {
    shards: Vec<Box<dyn Database>>,
    hash_ring: ConsistentHash,
}

impl ShardingManager {
    pub async fn write(&self, user_id: &str, op: WriteOp) -> Result<()> {
        let shard_id = self.hash_ring.get_node(user_id)?;
        self.shards[shard_id].execute(&op).await
    }
    
    pub async fn query(&self, user_id: &str, query: Query) -> Result<Vec<Row>> {
        let shard_id = self.hash_ring.get_node(user_id)?;
        self.shards[shard_id].query(&query).await
    }
}
```

### 8.2 Resharding Process

When adding new shard:
1. Start new database replica
2. Copy range of data (consistent hash)
3. Update routing rules
4. Verify consistency
5. Delete old data

---

## 9. Implementation Timeline

| Phase | v1.1.0 | Start | Duration | Priority |
|---|---|---|---|---|
| 11: Caching + Redis | v1.1.0-alpha | June 1 | 3 weeks | 🔴 HIGH |
| 12: PostgreSQL | v1.1.0-alpha | June 15 | 4 weeks | 🔴 HIGH |
| 13: WebSocket Batching | v1.1.0-beta | July 1 | 2 weeks | 🟡 MEDIUM |
| 14: OAuth2/MFA | v1.1.0-beta | July 15 | 3 weeks | 🟡 MEDIUM |
| 15: Jaeger Tracing | v1.1.0-RC | August 1 | 2 weeks | 🟡 MEDIUM |
| 16: Multi-Region | v1.1.0-RC | August 15 | 3 weeks | 🟢 LOW |
| 17: gRPC Protocol | v1.2.0 | Sept 1 | 4 weeks | 🟢 LOW |
| 18: Sharding | v1.2.0+ | TBD | TBD | 🟢 LOW |

---

## 10. Success Criteria

### v1.1.0-GA Targets

✅ **Performance:**
- 50,000+ req/sec throughput (10-instance cluster)
- p95 latency < 10ms sustained
- 60%+ cache hit ratio

✅ **Scalability:**
- Horizontal scaling to 20+ instances
- PostgreSQL production deployment
- Multi-region replication

✅ **Features:**
- Redis caching layer
- OAuth2/OIDC authentication
- Distributed tracing (Jaeger)
- WebSocket message batching
- Advanced monitoring dashboards

✅ **Reliability:**
- 99.99% uptime target
- Automatic failover
- Point-in-time recovery

✅ **Documentation:**
- v1.0.1 → v1.1.0 migration guide
- Multi-region deployment guide
- OAuth2/OIDC setup instructions
- Sharding architecture doc

---

## 11. Risk Mitigation

| Risk | Impact | Mitigation |
|---|---|---|
| Redis crash | Cache miss spike | Use Redis Cluster, persistence |
| PostgreSQL migration failure | Downtime | Dual-write testing, rollback plan |
| WebSocket batching issues | Message loss | Comprehensive testing, batching verification |
| OAuth provider downtime | Auth failures | Local fallback to token auth |
| Multi-region sync lag | Data inconsistency | Eventual consistency model, user education |

---

## 12. Community Feedback

- Collect user feedback via GitHub Discussions
- Prioritize features based on usage patterns
- Beta testing program (alpha releases to select users)
- Monthly updates in blog/changelog

---

## Conclusion

**v1.1.0 Transformation:**
- SQLite → PostgreSQL (scalability)
- Single instance → Multi-region (availability)
- REST-only → REST + gRPC (flexibility)
- Token auth → Multi-auth (enterprise features)
- Basic monitoring → Observability stack (production-grade)

**Goal:** Make slskR enterprise-ready for organizations running 50K+ concurrent users and handling 100K+ req/sec across global infrastructure.
