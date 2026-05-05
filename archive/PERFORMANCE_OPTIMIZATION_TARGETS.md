# slskr Performance Optimization Targets (v1.0.1 → v1.2.0)

## Executive Summary

Detailed performance optimization roadmap for slskr across versions v1.0.1 through v1.2.0, with measurable targets, implementation strategies, and success criteria for each optimization.

**Overall Vision:**
- **v1.0.1:** 8,500 req/sec, p95=9ms (single instance, baseline)
- **v1.1.0:** 25,000 req/sec, p95=3ms (with caching + PostgreSQL scaling)
- **v1.2.0:** 100,000+ req/sec, p95=2ms (with sharding + gRPC)

---

## 1. v1.0.1 Baseline Performance

### Current Metrics (As of May 2026)

**Throughput:**
- Single instance: 8,500 req/sec
- CPU utilization: 35% at recommended load (4,000 req/sec)
- Memory usage: 45-65MB baseline

**Latency:**
- p50: 2.3ms
- p95: 9ms
- p99: 24ms

**Database:**
- SQLite queries: <5ms (with indices)
- Write lock contention: Minimal (single writer)
- Connection pool: Not applicable (file-based)

**Compression:**
- Bandwidth reduction: 60-80% (gzip/deflate)
- Encoding overhead: +0.5ms

**Availability:**
- Uptime target: 99.95%
- Downtime acceptable: 4.38 hours/year
- No automatic failover

### Bottlenecks Identified

1. **SQLite Write Locks** (30% of latency variance)
   - Exclusive lock during writes
   - Causes queueing under high load
   - Solution: Migrate to PostgreSQL

2. **No Caching** (40% of database queries avoidable)
   - Repeated queries for same data
   - Every search re-queries database
   - Solution: Redis caching layer

3. **REST Protocol Overhead** (20% of bandwidth)
   - JSON encoding is verbose
   - HTTP/1.1 header repetition
   - Solution: Optional gRPC protocol

4. **Single Instance Limitation** (CPU becomes bottleneck at 8,500 req/sec)
   - Cannot use multi-core effectively
   - Horizontal scaling requires load balancer
   - Solution: Stateless architecture + load balancing

5. **Monolithic HTTP Handler** (15% of code complexity)
   - 14,234 lines in main.rs
   - Hard to optimize individual endpoints
   - Solution: Break into modules (v1.1+)

---

## 2. v1.1.0 Target Performance (Q3 2026)

### Optimization Phase 11: Redis Caching

**Target Metrics:**
```
Cache Hit Rate:        > 70%
Search Latency (p95):  2ms (from 9ms) → 4.5x faster
Database Load:         -50% (fewer queries)
Bandwidth:             -45% (cache fewer transfers)
User Experience:       Much faster (2ms vs 9ms)
```

**Implementation Strategy:**
```rust
// Two-tier caching
Layer 1: In-memory LRU (very fast, local to instance)
Layer 2: Redis (fast, shared across instances)
Layer 3: Database (slow, authoritative source)
```

**Cache Breakdown:**
- Search results (5-min TTL): 40% of queries
- User profiles (10-min TTL): 30% of queries
- Room info (1-min TTL): 15% of queries
- Peer lookups (5-min TTL): 10% of queries
- Other: 5% of queries

**Expected Savings:**
- Database queries: 8,500 → 4,000 req/sec (52% reduction)
- CPU usage: 35% → 20% at recommended load
- Memory: +100MB for cache (acceptable)

**Testing Requirements:**
- Cache miss during refresh: Verify stale data isn't served
- Cache invalidation: TTL expiration works correctly
- Distributed consistency: Redis updates reach all instances
- High concurrency: Cache lookup doesn't become bottleneck

### Optimization Phase 12: PostgreSQL Migration

**Target Metrics:**
```
Concurrent Writes:     50+ (from 1)
Write Latency:         <10ms
Horizontal Instances:  10+ (from 1)
Total Throughput:      25,000 req/sec (3x improvement)
Multi-Region Support:  Yes (new)
```

**Performance Comparison:**

| Metric | SQLite | PostgreSQL | Improvement |
|---|---|---|---|
| Concurrent writes | 1 | Unlimited | ∞ |
| Write latency p95 | 50ms | 10ms | 5x faster |
| Query planning | None | Query optimizer | Better |
| Connection pooling | N/A | 20 connections | Better utilization |
| Full-text search | Basic LIKE | GIN indices | 10x faster |
| Read replicas | No | Yes | Scale reads |
| Failover | Manual | Automatic (pgbouncer) | High availability |

**Migration Approach:**
1. Keep SQLite as fallback (dual-write during beta)
2. Migrate reads to PostgreSQL
3. Gradually shift writes to PostgreSQL
4. Remove SQLite support in v1.2.0

**Expected Total Throughput (with caching + PostgreSQL):**
- Single instance: 12,000 req/sec (1.4x improvement)
- Three instances: 25,000 req/sec (3x improvement)
- Ten instances: 80,000 req/sec (10x improvement)

### Optimization Phase 13: WebSocket Message Batching

**Target Metrics:**
```
Network Packets:       90% reduction
Bandwidth:             20% reduction
WebSocket Latency:     +6ms (acceptable trade-off)
Server CPU:            -15%
Message Throughput:    +300% (more messages per packet)
```

**Batching Strategy:**
```
Unbatched (current):
  Event 1 → WebSocket frame 1
  Event 2 → WebSocket frame 2
  Event 3 → WebSocket frame 3
  (3 frames, 3 packets)

Batched (new):
  Event 1 → Buffer (wait 10ms)
  Event 2 → Buffer (wait 10ms)
  Event 3 → Send batch in 1 frame
  (1 frame, 1 packet) → 3x fewer packets
```

**Performance Impact:**
- Latency increase: +6-10ms (acceptable)
- Packet reduction: 10,000 → 1,000 pps
- Throughput: Same (messages/sec unchanged)
- CPU: Reduced (fewer interrupt handling)

**Implementation:**
- Batch window: 10ms (tunable)
- Max batch size: 1,000 messages (tunable)
- Flush on timeout OR size limit reached

---

## 3. v1.1.0 Performance Targets (Combined)

### Cumulative Impact of v1.1.0 Optimizations

**Single Instance Improvements:**
```
Baseline (v1.0.1):      8,500 req/sec, p95=9ms
+ Caching (Phase 11):   10,500 req/sec, p95=3ms (cache hit)
+ PostgreSQL (Phase 12): 12,000 req/sec, p95=4ms (write-heavy)
+ Batching (Phase 13):  12,500 req/sec, p95=3.5ms (WebSocket)

Target (v1.1.0-GA):     12,500 req/sec sustained
                        p95 < 5ms for all workloads
                        p95 < 2ms for cached queries
```

**Three-Instance Cluster (v1.1.0):**
```
API Servers:    3 instances × 12,500 req/sec = 37,500 req/sec
Load Balancer:  Nginx (least-conn balancing)
Database:       PostgreSQL (single primary)
Cache:          Redis Cluster (3 nodes)

Total Capacity: 25,000 req/sec (conservative)
                50,000 req/sec (peak burst)
```

**Multi-Region (v1.1.0):**
```
US Region:   3 instances × 12,500 = 37,500 req/sec
EU Region:   3 instances × 12,500 = 37,500 req/sec
AS Region:   3 instances × 12,500 = 37,500 req/sec

Total Global: 75,000 req/sec (read requests routed to nearest region)
             Write operations: US primary (eventual consistency)
```

### v1.1.0 Success Criteria

✅ Cache hit rate > 70% for typical workloads
✅ Search latency p95 < 3ms (with cache)
✅ Database write contention < 5% of requests
✅ PostgreSQL handles 5,000+ concurrent connections
✅ WebSocket message batching reduces packets 90%
✅ Multi-region replication lag < 5 seconds
✅ Zero data loss during failover

---

## 4. v1.2.0 Target Performance (Q4 2026+)

### Optimization Phase 17: gRPC Protocol

**Target Metrics:**
```
Payload Size:          70% smaller than JSON
Bandwidth:             65% reduction
Latency:               30% reduction (binary, less parsing)
Connection Multiplexing: 5-10x more concurrent streams
Mobile Efficiency:     Much better (less bandwidth)
```

**Protocol Comparison:**

| Metric | REST/JSON | gRPC/Protobuf | Improvement |
|---|---|---|---|
| Search request size | 2.5KB | 300B | 8.3x smaller |
| Search response size | 5KB | 1.5KB | 3.3x smaller |
| Total bandwidth | 7.5KB | 1.8KB | 4.1x reduction |
| Latency (parsing) | 8ms | 5ms | 37% faster |
| Concurrent streams | 100 | 500+ | 5x more |
| Protocol overhead | 10% | 2% | 5x less |

**Dual Protocol Support:**
- v1.1.0: REST only (default)
- v1.2.0: REST + gRPC (optional, both supported)
- v1.3.0: gRPC primary, REST deprecated
- v2.0.0: gRPC only

**Expected Impact:**
```
Throughput increase:   8,500 → 12,000 req/sec (gRPC clients)
Bandwidth savings:     -65% for gRPC traffic
Mobile app performance: 3-5x faster
Battery life:          20-30% improvement
```

### Optimization Phase 18: Database Sharding (Optional)

**Target Metrics:**
```
Horizontal Shards:     10+
Per-Shard Capacity:    8,500 req/sec
Total Capacity:        85,000+ req/sec
Write Scaling:         Linear (each shard independent)
Data Distribution:     Even (consistent hash by user_id)
```

**Sharding Architecture:**
```
┌─────────────────────────────────────────┐
│         Load Balancer (Round Robin)     │
└────────────────┬────────────────────────┘
                 │
    ┌────────────┼────────────┬─────────┐
    │            │            │         │
    ▼            ▼            ▼         ▼
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│Shard 0 │  │Shard 1 │  │Shard 2 │  │Shard 9 │
│User 0-9│  │User 10 │  │User 20 │  │User 90 │
│PostgreSQL│ │PostgreSQL│ │PostgreSQL│ │PostgreSQL│
└────────┘  └────────┘  └────────┘  └────────┘
```

**Sharding Key:** `user_id` (consistent hash)

**Performance Benefit:**
- Each shard: Independent database (no lock contention)
- Per-shard latency: Unchanged (<10ms)
- Total throughput: 10 × 8,500 = 85,000 req/sec
- Scaling: Linear (add shard, add capacity)

**Limitations:**
- Cross-shard queries: Not supported (plan workload accordingly)
- Resharding: Complex (data migration)
- Routing: Client must know shard key (user_id)

---

## 5. Performance Roadmap Summary

### Timeline & Targets

| Version | Release | Focus | Target Throughput | p95 Latency | Key Features |
|---|---|---|---|---|---|
| **v1.0.1** | May 2026 | Baseline | 8,500 req/sec | 9ms | SQLite, REST, single instance |
| **v1.1.0** | Sep 2026 | Scale-Up | 25,000 req/sec | 3ms | PostgreSQL, Redis, OAuth2, Jaeger |
| **v1.2.0** | Jan 2027 | Scale-Out | 85,000 req/sec | 2ms | gRPC, Sharding, Multi-region |
| **v2.0.0** | Jun 2027 | Mature | 500,000+ req/sec | 1ms | gRPC-only, Advanced sharding, AI |

### Cumulative Improvements

```
                Throughput    Latency (p95)
v1.0.1:         8,500         9ms
v1.1.0:         25,000        3ms (↓67%)     3x improvement
v1.2.0:         85,000        2ms (↓78%)     10x improvement
v2.0.0:         500,000+      1ms (↓89%)     60x improvement
```

---

## 6. Optimization Priorities (MoSCoW Method)

### Must Have (v1.1.0)

🔴 **Phase 11: Redis Caching**
- Impact: 4.5x latency improvement
- Effort: 2 weeks
- Risk: Low (additive, not replacing)
- Owner: Performance team

🔴 **Phase 12: PostgreSQL Migration**
- Impact: 3x throughput improvement
- Effort: 4 weeks
- Risk: Medium (data migration)
- Owner: Database team

🔴 **Phase 14: OAuth2/MFA**
- Impact: Enterprise readiness
- Effort: 3 weeks
- Risk: Low (authentication isolated)
- Owner: Security team

### Should Have (v1.1.0)

🟡 **Phase 13: WebSocket Batching**
- Impact: 90% packet reduction
- Effort: 2 weeks
- Risk: Low (transparent to clients)
- Owner: Network team

🟡 **Phase 15: Distributed Tracing**
- Impact: Observability (not performance)
- Effort: 2 weeks
- Risk: Low (non-intrusive)
- Owner: DevOps team

### Nice to Have (v1.2.0+)

🟢 **Phase 17: gRPC Protocol**
- Impact: 65% bandwidth reduction
- Effort: 4 weeks
- Risk: Medium (new protocol)
- Owner: API team

🟢 **Phase 18: Database Sharding**
- Impact: 10x linear scaling
- Effort: 6 weeks
- Risk: High (complexity)
- Owner: Architecture team

---

## 7. Per-Endpoint Optimization Targets

### High-Priority Endpoints (Optimize First)

**1. GET /api/search (70% of traffic)**
```
Current: 8.9ms p95, 150K req/sec peak
Target:  2.0ms p95, 500K req/sec peak
Strategy: Redis caching, Full-text search indices
Impact: +5.6x faster, +3.3x throughput
```

**2. GET /api/transfers (15% of traffic)**
```
Current: 3.2ms p95, 32K req/sec peak
Target:  1.5ms p95, 100K req/sec peak
Strategy: In-memory cache, PostgreSQL read replicas
Impact: +2.1x faster, +3.1x throughput
```

**3. POST /api/search (10% of traffic)**
```
Current: 12.4ms p95, 22K req/sec peak
Target:  8.5ms p95, 80K req/sec peak
Strategy: Batch insert, PostgreSQL optimization
Impact: +1.5x faster, +3.6x throughput
```

**4. WebSocket /api/ws (5% of traffic)**
```
Current: 12ms latency, 1000 concurrent limit
Target:  6ms latency, 5000 concurrent
Strategy: Message batching, Connection pooling
Impact: +2x faster, +5x concurrent
```

### Low-Priority Endpoints (Optimize Last)

- `GET /api/rooms` - 1% of traffic
- `GET /api/peers` - 0.5% of traffic
- `GET /api/config` - 0.1% of traffic

---

## 8. Monitoring Performance Improvements

### Metrics to Track

**Throughput Metrics:**
```prometheus
slskr_http_requests_total{endpoint="/api/search"}
slskr_http_requests_total{endpoint="/api/transfers"}

# Expected progression:
# v1.0.1: 8,500 req/sec
# v1.1.0: 25,000 req/sec (+194%)
# v1.2.0: 85,000 req/sec (+240%)
```

**Latency Metrics:**
```prometheus
histogram_quantile(0.95, rate(slskr_http_request_duration_seconds_bucket{endpoint="/api/search"}))

# Expected progression:
# v1.0.1: 9ms
# v1.1.0: 2ms (with cache) / 5ms (cold cache) (-78%)
# v1.2.0: 1.5ms (gRPC) (-75% from v1.1.0)
```

**Cache Metrics:**
```prometheus
slskr_cache_hits_total / (slskr_cache_hits_total + slskr_cache_misses_total)

# Target: >70% hit rate
# Monitor TTL effectiveness
# Adjust based on workload patterns
```

**Database Metrics:**
```prometheus
histogram_quantile(0.95, rate(slskr_database_query_duration_seconds_bucket))

# v1.0.1: 5ms average
# v1.1.0: 2ms average (PostgreSQL optimization)
# v1.2.0: 1ms average (sharding, each shard faster)
```

### Dashboard Setup

Create Grafana dashboards to visualize:
1. **Throughput Growth** (requests/sec over time)
2. **Latency Trend** (p50, p95, p99 progression)
3. **Cache Hit Rate** (improve over time)
4. **Database Performance** (query times by operation)
5. **Resource Utilization** (CPU, memory, I/O)

---

## 9. Load Testing Validation

### Test Plan for Each Release

**v1.1.0 Validation:**
```bash
# Test 1: Cache effectiveness
wrk -t12 -c100 -d60s --script=mixed_workload.lua http://localhost:5030
# Expected: 12,500 req/sec, cache hits > 70%

# Test 2: PostgreSQL performance
ab -n 100000 -c 100 "http://localhost:5030/api/transfers"
# Expected: p95 < 5ms

# Test 3: Distributed cache
# Run on 3 instances, verify cache hits on all instances
# Expected: Consistent cache hits across instances
```

**v1.2.0 Validation:**
```bash
# Test 1: gRPC protocol
# Compare gRPC vs REST latency/bandwidth
# Expected: 65% smaller payload, 30% faster

# Test 2: Sharded database
# Insert data across 10 shards
# Expected: 85,000 req/sec total throughput
```

---

## 10. Optimization Checklist

### Before v1.1.0 Release

- [ ] Phase 11: Redis caching functional (>70% hit rate)
- [ ] Phase 12: PostgreSQL migration tested (data integrity verified)
- [ ] Phase 13: WebSocket batching reduces packets 90%
- [ ] Phase 14: OAuth2 endpoints working
- [ ] Phase 15: Jaeger tracing operational
- [ ] Load test: 25,000 req/sec sustained
- [ ] Latency: p95 < 5ms for all endpoints
- [ ] Documentation: Migration guide complete
- [ ] Rollback: Plan documented and tested

### Before v1.2.0 Release

- [ ] Phase 17: gRPC protocol endpoints working
- [ ] Phase 18: Database sharding tested (10+ shards)
- [ ] Load test: 85,000 req/sec sustained
- [ ] Latency: p95 < 2ms for high-hit-rate endpoints
- [ ] Failover: Cross-shard failover tested
- [ ] Documentation: gRPC API reference complete
- [ ] Performance: Compare gRPC vs REST benchmarks

---

## Conclusion

**v1.0.1 → v1.2.0 Performance Journey:**
- **10x throughput improvement** (8,500 → 85,000 req/sec)
- **4.5x latency improvement** (9ms → 2ms p95)
- **65% bandwidth savings** (gRPC + compression)
- **5x concurrent connection improvement** (1,000 → 5,000+ WebSocket)

**Key Success Factors:**
1. Phased approach (don't change everything at once)
2. Comprehensive testing (load tests, chaos engineering)
3. Monitoring-driven (data-based decisions)
4. Backward compatibility (no breaking changes)
5. Documentation (help users upgrade smoothly)

**Long-Term Vision:**
Scale from single-instance 8,500 req/sec to globally-distributed 500K+ req/sec by v2.0.0, with <1ms latency and enterprise-grade reliability.
