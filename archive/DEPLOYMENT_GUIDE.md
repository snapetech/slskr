# slskr v1.0.1 - Production Deployment Guide

## Overview
slskr is a fully-featured Soulseek network daemon with comprehensive REST API (298+ endpoints), WebSocket/SignalR real-time features, SQLite persistence, and production-grade security controls.

## Architecture
- **HTTP Server**: Axum web framework (replacing hand-rolled router)
- **Database**: SQLite with sqlx async driver
- **WebSocket**: Native WebSocket + Server-Sent Events (SSE)
- **Real-Time**: SignalR hubs for browser clients
- **Security**: CORS, CSRF, rate limiting, input validation
- **Observability**: Request ID tracking, structured logging

## Installation

### Prerequisites
- Rust 1.70+ (or provided binary)
- Linux/macOS/Windows x86_64
- Network connectivity (Soulseek protocol)
- 50MB+ free disk space
- 2GB+ RAM recommended

### Build from Source
```bash
cd /home/keith/Documents/code/slskr
cargo build --release
./target/release/slskr daemon
```

### Docker Deployment
```bash
# Dockerfile (provided)
docker build -t slskr:1.0.1 .
docker run -d \
  -p 5030:5030 \
  -p 6346:6346 \
  -v slskr-data:/data \
  -e SLSKR_HTTP_BIND=0.0.0.0:5030 \
  -e SLSKR_STATE_DIR=/data \
  slskr:1.0.1
```

## Configuration

### Environment Variables
```bash
# HTTP Server
export SLSKR_HTTP_BIND=127.0.0.1:5030  # Listen address:port

# State Persistence
export SLSKR_STATE_DIR=/var/lib/slskr   # Data directory

# Database
export SLSKR_DB_PATH=/var/lib/slskr/slskr.db  # SQLite database

# Security
export SLSKR_API_TOKEN=your-secure-token  # API authentication
export SLSKR_CORS_ORIGINS=https://example.com  # CSV list

# Soulseek Network
export SLSK_SERVER_ADDRESS=server.slsknet.org:2242
export SLSK_USERNAME=your-username
export SLSK_PASSWORD=your-password

# Logging
export RUST_LOG=info,slskr=debug
export RUST_BACKTRACE=1
```

### Configuration File
Create `~/.config/slskr/config.toml`:
```toml
[app]
http_bind = "127.0.0.1:5030"
state_dir = "/var/lib/slskr"

[network]
server_address = "server.slsknet.org:2242"
listen_port = 3333
username = "your-username"
password = "your-password"

[auth]
api_token = "your-secure-token"
disabled = false

[security]
cors_origins = ["https://example.com", "http://localhost:3001"]
csrf_enabled = true
rate_limit_enabled = true
```

## API Access

### Authentication
```bash
# Bearer token in Authorization header
curl -H "Authorization: Bearer YOUR_API_TOKEN" \
  http://localhost:5030/api/health

# Or cookie-based for web UI
curl -b "slskr.session=YOUR_API_TOKEN" \
  http://localhost:5030/api/health
```

### Example Requests
```bash
# Check server health
curl http://localhost:5030/api/health

# Get API capabilities
curl http://localhost:5030/api/capabilities

# List searches
curl -H "Authorization: Bearer TOKEN" \
  http://localhost:5030/api/searches

# Create search
curl -X POST http://localhost:5030/api/searches \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"artist:Album"}'

# WebSocket connection
wscat -c ws://localhost:5030/ws/transfers \
  --header "Authorization: Bearer TOKEN"
```

## Database Initialization

SQLite database is automatically created on first run with schema:
- searches
- transfers
- messages
- user_stats
- rooms
- webhooks
- webhook_logs

Indices are automatically created for performance.

## Monitoring

### Health Endpoints
- `GET /api/health`: Service health
- `GET /api/version`: Version information
- `GET /api/telemetry`: Runtime metrics

### Logs
```bash
# Follow daemon logs
journalctl -u slskr -f

# Or from stdout if running directly
./target/release/slskr daemon 2>&1 | tee slskr.log
```

### Metrics
- Request duration: Response headers include timing
- Rate limit stats: Check RateLimit-* headers
- Cache hit ratio: Monitor Cache-Control headers

## Performance Tuning

### Database
```bash
# Tune SQLite connection pool
SLSKR_DB_POOL_SIZE=5  # Default
SLSKR_DB_POOL_TIMEOUT=30  # Seconds
```

### Network
```bash
# Transfer settings
SLSKR_TRANSFER_MAX_ACTIVE=10
SLSKR_TRANSFER_QUEUE_SIZE=100
```

### Rate Limiting
```bash
# Per-IP limits
SLSKR_RATE_LIMIT_ANONYMOUS=100  # per minute
SLSKR_RATE_LIMIT_AUTHENTICATED=1000  # per minute
```

## Security Hardening

### HTTPS/TLS
```bash
# Use reverse proxy (Nginx recommended)
server {
    listen 443 ssl http2;
    server_name api.example.com;
    
    ssl_certificate /etc/ssl/certs/cert.pem;
    ssl_certificate_key /etc/ssl/private/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    
    location / {
        proxy_pass http://localhost:5030;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Firewall
```bash
# Only expose API port to trusted sources
sudo ufw allow from 192.168.1.0/24 to any port 5030
sudo ufw allow from 0.0.0.0/0 to any port 443  # HTTPS only
```

### API Security
- Rotate API tokens regularly
- Use strong, random tokens (32+ chars)
- Monitor for suspicious access patterns
- Enable CORS only for trusted origins
- Set rate limits based on expected load

## Scaling

### Horizontal Scaling
For multiple instances:
1. Use shared database (configure with DATABASE_URL)
2. Deploy behind load balancer (Nginx, HAProxy)
3. Use distributed session storage
4. Share WebSocket subscriptions via Redis

### Vertical Scaling
- Increase connection pool size
- Tune Tokio thread count
- Use SSD for database
- Allocate 4GB+ RAM

## Troubleshooting

### Common Issues

**Connection Refused**
```bash
# Check port availability
lsof -i :5030
# Check firewall
ufw status
```

**High Memory Usage**
```bash
# Check active transfers
curl http://localhost:5030/api/transfers
# Consider reducing TRANSFER_MAX_ACTIVE
```

**Slow Queries**
```bash
# Check database file size
du -h /var/lib/slskr/slskr.db
# Run optimization
sqlite3 /var/lib/slskr/slskr.db "VACUUM;"
```

**Rate Limit Errors**
```bash
# Check RateLimit headers in response
curl -i http://localhost:5030/api/health
# Adjust limits if needed
```

## Backup & Recovery

### Daily Backup
```bash
#!/bin/bash
BACKUP_DIR=/backups/slskr
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
sqlite3 /var/lib/slskr/slskr.db ".backup $BACKUP_DIR/slskr_$TIMESTAMP.db"
```

### Restore from Backup
```bash
sqlite3 /var/lib/slskr/slskr.db ".restore /backups/slskr/slskr_20260504.db"
```

## Monitoring Services

### Systemd Service File
Create `/etc/systemd/system/slskr.service`:
```ini
[Unit]
Description=slskr Soulseek Daemon
After=network.target

[Service]
Type=simple
User=slskr
WorkingDirectory=/opt/slskr
Environment="SLSKR_STATE_DIR=/var/lib/slskr"
Environment="RUST_LOG=info"
ExecStart=/opt/slskr/slskr daemon
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable slskr
sudo systemctl start slskr
sudo systemctl status slskr
```

## Updates

### Version Checking
```bash
curl http://localhost:5030/api/application/version/latest
```

### Upgrade Process
1. Stop daemon: `systemctl stop slskr`
2. Backup database: `cp slskr.db slskr.db.bak`
3. Deploy new binary
4. Start daemon: `systemctl start slskr`
5. Monitor logs: `journalctl -f`

## Support & Debugging

### Enable Debug Logging
```bash
export RUST_LOG=debug,slskr=trace
./target/release/slskr daemon
```

### Request Tracing
All requests include unique X-Request-ID header:
```bash
curl -v http://localhost:5030/api/health 2>&1 | grep X-Request-ID
```

Use request ID to correlate logs:
```bash
journalctl --grep "REQUEST_ID_VALUE"
```

## Links

- **Repository**: https://github.com/... (slskr)
- **Issues**: https://github.com/.../issues
- **WebUI**: http://localhost:3001 (development)
- **API Docs**: http://localhost:5030/api/capabilities

## Version Information

- **Version**: 1.0.1
- **Release Date**: 2026-05-04
- **Endpoints**: 298+ (102% of specification)
- **Status**: Production Ready

---

For more information, see QUICK_START.md and PHASE_COMPLETION_REPORT.md
