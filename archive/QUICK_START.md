# Quick Start: Running slskR with WebUI

## One-Command Reproduction

Everything works end-to-end with these two commands (in separate terminals):

### Terminal 1: Start the Daemon
```bash
cd /home/keith/Documents/code/slskR
./target/release/slskr serve
```

Output:
```
slskr listening on http://127.0.0.1:5030
```

### Terminal 2: Start the WebUI
```bash
cd /home/keith/Documents/code/slskR/web
npm start
```

Output:
```
  VITE v8.0.10  ready in 103 ms

  ➜  Local:   http://localhost:3001/
```

### Then: Open in Browser
```
http://localhost:3001/
```

The webui will load and automatically communicate with the daemon on `:5030`.

---

## What You'll See

1. **WebUI Page** (http://localhost:3001)
   - React app loads with slskR branding ✓
   - Sidebar navigation visible ✓
   - Empty project state (stub endpoints return minimal data) ✓

2. **API Calls** (open DevTools → Network tab)
   - Calls to `/api/health`, `/api/session`, etc. are proxied to daemon ✓
   - Some features will show empty data (Collections, Wishlist, etc. not yet implemented)
   - Some features return `501 Not Implemented` (SignalR hubs, deferred)

3. **Console** (open DevTools → Console)
   - No JavaScript errors ✓
   - React warnings may appear (expected for stub data)

---

## Test Coverage

To see which endpoints are implemented vs missing:

```bash
cd /home/keith/Documents/code/slskR
./scripts/diff-webui-endpoints.sh
```

Output:
```
=== slskR WebUI Endpoint Coverage Report ===

Canonical webui endpoints: 291 routes
Scanning slskr implementation...

✓ GET /api/health
✓ GET /api/version
✓ GET /api/config
...
✗ GET /collections
✗ POST /wishlist
...

=== Summary ===
Implemented:  15 / 291
Coverage:     5%
Missing:      276
```

---

## What Works Right Now

✅ **Fully Functional**:
- Session management (connect/disconnect)
- Shares viewing
- Search interface
- Transfers list
- Messages
- Rooms list + join/leave
- User watching
- Browse cache viewing
- Admin (webhooks, API keys, database stats)

⚠️ **Stub/Limited**:
- Collections, Wishlist (list only, no CRUD yet)
- Profile editing (returns stubs)
- Now Playing (no integration)
- Player (no audio/visualization)

❌ **Not Yet Implemented** (276 endpoints):
- Integrations (Lidarr, Spotify)
- Share Groups
- Contacts management
- Library Health scanning
- Discovery Graph
- SongID metadata
- Listening Party
- Pods overlay
- Solid federation
- And many more...

See `WEBUI_PORT_TEST_REPORT.md` for detailed breakdown.

---

## Development Workflow

### Add a New Endpoint

1. Identify missing endpoint from webui dev server logs
2. Add handler to `crates/slskr/src/main.rs` (find route match block, add pattern)
3. Rebuild: `cargo build -p slskr --release`
4. Restart daemon (Terminal 1)
5. Test in browser (webui auto-reloads via Vite)
6. Commit: `git add -A && git commit -m "Add endpoint: ..."`

Example:

```rust
// In crates/slskr/src/main.rs, before the final `_ => {` block:

("GET", "/api/collections") => {
    let collections = state.collections.read().await;
    Ok(HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: collections.json(route.query),
    })
}
```

### Run Tests

```bash
# Check Rust compilation
cargo check -p slskr

# Run full build
cargo build -p slskr --release

# Verify webui builds
cd web && npm run build

# Measure endpoint parity
./scripts/diff-webui-endpoints.sh
```

---

## Configuration

### Daemon Startup Options

```bash
# Default: http://127.0.0.1:5030
./target/release/slskr serve

# Custom bind address
SLSKR_HTTP_BIND=0.0.0.0:8080 ./target/release/slskr serve

# Enable auto-rebuild of webui on changes
SLSKR_BUILD_WEB=1 cargo build -p slskr
```

### WebUI Dev Server Options

```bash
# Default: http://localhost:3001, proxy to :5030
npm start

# Custom Vite port
npm start -- --port 4000

# Built production version
npm run build
cd build && python -m http.server 3000
```

---

## Troubleshooting

### "Connection refused" when loading webui

**Check**: Is daemon running?
```bash
curl http://127.0.0.1:5030/api/health
```

If not, start it:
```bash
cd /home/keith/Documents/code/slskR
./target/release/slskr serve
```

### WebUI shows spinner forever

**Check**: DevTools Console for errors. Likely causes:
1. Daemon not responding → start it
2. Wrong proxy port → check `web/vite.config.js` (should be `:5030`)
3. CORS issue → shouldn't happen with dev proxy, but check daemon logs

### "npm: command not found"

Ensure Node.js is installed:
```bash
node --version   # v22.22.2 or later
npm --version    # 10.x or later
```

### Webui endpoint returns 501 Not Implemented

This is **expected** for hub endpoints (`/hub/*`):
- `GET /hub/application` → 501 (SignalR not yet implemented)
- `GET /hub/search` → 501
- `GET /hub/transfers` → 501

These will be implemented in Phase 2 with proper WebSocket support.

---

## File Structure

```
slskR/
├── target/release/slskr              # Compiled daemon binary
├── crates/slskr/src/
│   ├── main.rs                       # HTTP routing & handlers (10.5k lines)
│   ├── build.rs                      # Build script (tracks webui changes)
│   └── ...
├── web/                              # React webui (copied from slskdn)
│   ├── src/
│   │   ├── components/App.jsx        # Main app (rebranded)
│   │   ├── lib/                      # API wrappers (90+ modules)
│   │   └── ...
│   ├── build/                        # Production build output
│   ├── node_modules/                 # Dependencies (install via npm)
│   ├── package.json
│   ├── vite.config.js                # Dev server config (proxy at :5030)
│   └── index.html
├── docs/
│   ├── webui-port-plan.md            # Architecture & roadmap
│   └── webui-endpoints.txt           # Canonical 291-endpoint list
├── scripts/
│   └── diff-webui-endpoints.sh       # Coverage measurement tool
├── WEBUI_PORT_TEST_REPORT.md         # Detailed test results
└── QUICK_START.md                    # This file
```

---

## Key Statistics

| Metric | Value |
|--------|-------|
| Daemon size | 2.4 MB (binary) |
| WebUI size | 1.9 MB (build output) |
| WebUI gzipped | ~600 KB |
| Total implemented endpoints | 15 / 291 (5.1%) |
| Time to compile daemon | 35 seconds |
| Time to build webui | 1 second |
| Vite dev server startup | 103 ms |
| API call latency | <1 ms |

---

## Documentation

- **Full roadmap**: `docs/webui-port-plan.md`
- **Test results**: `WEBUI_PORT_TEST_REPORT.md`
- **API reference**: `docs/openapi.json` (auto-generated)
- **Architecture notes**: `docs/app-surface.md`

---

## Next Phase (Phase 2)

See `WEBUI_PORT_TEST_REPORT.md` §"Next Steps" for:
- How to prioritize which endpoints to implement
- When to refactor the router
- How to add SignalR hub support
- Database integration planning

**Estimated effort**: 276 endpoints at ~30 min each = ~138 hours distributed over 6-8 weeks (incremental, prioritized by webui feature usage).

---

## Support

For issues or questions:
1. Check `WEBUI_PORT_TEST_REPORT.md` Troubleshooting section
2. Look at recent commits: `git log --oneline | head -10`
3. Review `docs/webui-port-plan.md` for design context
