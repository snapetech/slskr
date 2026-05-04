# Rate Limiting

## Overview

slskR implements per-user and per-IP rate limiting to protect the API from abuse and ensure fair resource allocation across clients.

## Configuration

Rate limiting is configured at startup with the following defaults:

- **Anonymous (IP-based) limit:** 1,000 requests per 60 seconds
- **Authenticated (user-based) limit:** 5,000 requests per 60 seconds

These limits can be modified in `crates/slskr/src/config.rs`:
- `AppConfig.api_rate_limit_anonymous`
- `AppConfig.api_rate_limit_authenticated`

## How It Works

### Request Classification

1. **Authenticated Requests** - Requests with a valid `Authorization: Bearer <token>` header are rate-limited by the authenticated token/user
2. **Anonymous Requests** - Requests without authentication are rate-limited by source IP address

### Rate Limit Enforcement

When a client exceeds its configured limit within the rate limit window:

- **Status:** `429 Too Many Requests`
- **Response:** `{"error":"rate limited"}`

Once the rate limit window resets, the client can make requests again.

### Sliding Window Implementation

The rate limiter uses a sliding window approach:
- Each client (user or IP) has a time window of 60 seconds
- When the window expires, the request count resets
- Request tracking is separate for each unique identifier

## Architecture

### Components

1. **RateLimiter** (`crates/slskr/src/rate_limit.rs`)
   - Tracks requests per user and per IP
   - Provides `check_rate_limit()` for enforcement
   - Provides `get_remaining()` and `get_reset_time()` for client info

2. **RateLimitConfig** 
   - `max_requests_anonymous`: Limit for IP-based requests
   - `max_requests_authenticated`: Limit for user-based requests
   - `window_seconds`: Duration of rate limit window (default: 60)
   - `enabled`: Can be disabled for testing

3. **Integration Points**
   - `AppState.rate_limiter` - Initialized at startup
   - `handle_http_connection()` - Checks rate limits before routing requests
   - Returns 429 status when limit exceeded

## Example Usage

### Authenticated Request Within Limit
```bash
curl -H "Authorization: Bearer mytoken" \
  http://localhost:5030/api/stats
# Response: 200 OK with stats data
```

### Anonymous Request Within Limit
```bash
curl http://localhost:5030/api/health
# Response: 200 OK
```

### Request Exceeding Limit
```bash
# After 1000 requests from same IP in 60 seconds
curl http://localhost:5030/api/health
# Response: 429 Too Many Requests
# Body: {"error":"rate limited"}
```

## Testing

Rate limiting is fully tested with unit tests:

```bash
cargo test rate_limit --lib
```

Tests cover:
- Per-IP enforcement
- Per-user enforcement
- Different IPs/users track separately
- Disabled rate limiting
- Remaining request tracking
- Reset time calculation

## Future Enhancements

Potential improvements for v2:

1. **RateLimit Response Headers** - Add standard headers:
   - `RateLimit-Limit`: Maximum requests allowed
   - `RateLimit-Remaining`: Requests remaining in window
   - `RateLimit-Reset`: Seconds until window resets

2. **Granular Limits** - Different limits per endpoint or operation type

3. **Burst Allowance** - Higher limits for brief bursts with automatic backoff

4. **Redis Backend** - Distributed rate limiting across multiple instances

## Configuration Reference

```toml
# In slskr.toml (future support)
[api]
rate_limit_anonymous = 1000      # requests per window
rate_limit_authenticated = 5000   # requests per window
rate_limit_window_seconds = 60    # window duration
rate_limit_enabled = true         # enable/disable
```
