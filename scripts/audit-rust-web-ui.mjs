#!/usr/bin/env node
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { extname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('../web/node_modules/playwright');

const repoRoot = resolve(new URL('..', import.meta.url).pathname);
const distDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_DIST || 'target/slskr-web');
const auditDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_AUDIT_DIR || 'target/ux-audit');

const routes = [
  ['/searches', 'Search', 'searches'],
  ['/discovery-graph', 'Discovery Graph', 'discovery-graph'],
  ['/playlist-intake', 'Playlist Intake', 'playlist-intake'],
  ['/wishlist', 'Wishlist', 'wishlist'],
  ['/downloads', 'Downloads', 'downloads'],
  ['/uploads', 'Uploads', 'uploads'],
  ['/messages', 'Messages', 'messages'],
  ['/users', 'Users', 'users'],
  ['/contacts', 'Contacts', 'contacts'],
  ['/solid', 'Solid', 'solid'],
  ['/collections', 'Collections', 'collections'],
  ['/sharegroups', 'Share Groups', 'sharegroups'],
  ['/shared', 'Shared with Me', 'shared'],
  ['/browse', 'Browse', 'browse'],
  ['/system', 'System', 'system'],
];

const contentTypes = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.html', 'text/html; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.wasm', 'application/wasm'],
]);

const json = (body) => ({
  body: JSON.stringify(body),
  contentType: 'application/json',
  status: 200,
});

const mockBody = (path) => {
  if (path.includes('/health')) return { service: 'slskr', status: 'ok' };
  if (path.includes('/version')) return { name: 'slskr', version: '0.0.0' };
  if (path.includes('/application')) return { pendingRestart: false, relay: { enabled: false } };
  if (path.includes('/server')) return { isConnected: true, isLoggedIn: true, username: 'audit-user' };
  if (path.includes('/nowplaying')) return { filename: 'audit.flac', peer: 'dj-audit', state: 'playing' };
  if (path.includes('/transfers/speeds')) return { download: 98304, upload: 24576 };
  if (path.includes('/transfers/downloads')) {
    return [{ filename: 'audit-track.flac', peer: 'dj-audit', progress: 0.58, state: 'Active', size: 42152704 }];
  }
  if (path.includes('/transfers/uploads')) {
    return [{ filename: 'shared-track.flac', peer: 'listener-audit', progress: 0.32, state: 'Queued', size: 35127296 }];
  }
  if (path.includes('/searches/1/responses') || path.includes('/searches/:id/responses')) {
    return [{ filename: 'Artist - Audit.flac', username: 'peer-audit', size: 41800000, bitrate: 1011, queue: 0, speed: 512000 }];
  }
  if (path.includes('/searches/records') || path.endsWith('/searches')) {
    return [{ id: 1, searchText: 'Artist Audit', state: 'Complete', responseCount: 1 }];
  }
  if (path.includes('/wishlist')) {
    return [{ id: 'want-1', searchText: 'rare audit pressing', enabled: true, autoDownload: false, resultCount: 3 }];
  }
  if (path.includes('/conversations')) {
    return [{ username: 'chat-audit', unread: 1, lastMessage: 'Still available?', lastMessageAt: '2026-05-06T12:00:00Z' }];
  }
  if (path.includes('/rooms')) {
    return [{ name: 'audit-room', userCount: 7 }];
  }
  if (path.includes('/users/') && path.includes('/browse')) {
    return { username: 'peer-audit', folders: [{ name: 'Music', files: [{ filename: 'audit.flac', size: 1234 }] }] };
  }
  if (path.includes('/users')) {
    return [{ username: 'peer-audit', status: 'Online', privileged: true, files: 128 }];
  }
  if (path.includes('/contacts')) {
    return [{ username: 'friend-audit', status: 'Online', group: 'Friends', note: 'trusted' }];
  }
  if (path.includes('/solid/status')) {
    return { enabled: true, webId: 'https://audit.example/profile/card#me', storage: 'ready' };
  }
  if (path.includes('/collections')) {
    return [{ id: 'collection-1', title: 'Audit Collection', itemCount: 2, owner: 'audit-user' }];
  }
  if (path.includes('/sharegroups')) {
    return [{ id: 'group-1', name: 'Audit Group', members: 2, permissions: 'read,stream' }];
  }
  if (path.includes('/shared')) {
    return [{ id: 'grant-1', owner: 'friend-audit', title: 'Shared Audit', permissions: 'stream', expiresAt: null }];
  }
  if (path.includes('/source-providers')) return [{ id: 'provider-1', name: 'MusicBrainz', enabled: true }];
  if (path.includes('/jobs')) return [{ id: 'job-1', type: 'scan', state: 'Complete' }];
  if (path.includes('/shares')) return { roots: 1, files: 128, scanState: 'Idle' };
  if (path.includes('/database/stats')) return { tracks: 128, peers: 7 };
  if (path.includes('/logs')) return [{ level: 'info', message: 'audit log' }];
  if (path.includes('/telemetry') || path.includes('/metrics')) return { uptimeSeconds: 60 };
  return {};
};

const startStaticServer = async () => {
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url || '/', 'http://127.0.0.1');
      let filePath = join(distDir, decodeURIComponent(url.pathname));
      if (url.pathname === '/' || !existsSync(filePath)) filePath = join(distDir, 'index.html');
      const body = await readFile(filePath);
      response.writeHead(200, { 'content-type': contentTypes.get(extname(filePath)) || 'application/octet-stream' });
      response.end(body);
    } catch (error) {
      response.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
      response.end(error?.message || 'static server error');
    }
  });
  await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
  return server;
};

if (process.env.SLSKR_RUST_WEB_AUDIT_SKIP_BUILD !== '1') {
  const build = spawnSync('scripts/build-rust-web.sh', { cwd: repoRoot, stdio: 'inherit' });
  if (build.status !== 0) process.exit(build.status || 1);
}

const server = await startStaticServer();
await mkdir(auditDir, { recursive: true });
const { port } = server.address();
const browser = await chromium.launch({ headless: process.env.HEADLESS !== 'false' });
const audit = {
  errors: [],
  generatedAt: new Date().toISOString(),
  routes: [],
};

try {
  for (const [path, title, slug] of routes) {
    for (const viewport of [
      { name: 'desktop', width: 1440, height: 1000 },
      { name: 'mobile', width: 390, height: 844 },
    ]) {
      const page = await browser.newPage({ viewport });
      const pageErrors = [];
      page.on('pageerror', (error) => pageErrors.push(error.message));
      page.on('console', (message) => {
        if (message.type() === 'error' && !message.text().includes('Failed to load resource')) {
          pageErrors.push(message.text());
        }
      });
      await page.route('**/api/v0/**', async (route) => route.fulfill(json(mockBody(new URL(route.request().url()).pathname))));
      await page.goto(`http://127.0.0.1:${port}${path}`, { waitUntil: 'networkidle' });
      await page.screenshot({ fullPage: true, path: join(auditDir, `${slug}-${viewport.name}.png`) });

      const heading = await page.locator('.slskr-page-header h2').innerText();
      const developerOpen = await page.locator('.slskr-diagnostics').evaluate((node) => node.hasAttribute('open'));
      const bodyText = await page.locator('body').innerText();
      const primaryActionCount = await page.locator('.slskr-toolbar-command, .slskr-native-command-row button, .slskr-native-panel-actions button').count();
      const nativeWorkspaceCount = await page.locator('.slskr-native-workspace').count();
      const inspectorCount = await page.locator('#slskr-native-inspector').count();
      const rowCount = await page.locator('[data-slskr-native-select]').count();
      let selectedTitle = '';
      let selectedStatus = '';
      let toastText = '';
      let confirmationVisible = false;
      if (rowCount > 0) {
        await page.locator('[data-slskr-native-select]').first().focus();
        await page.keyboard.press('Enter');
        selectedTitle = await page.locator('[data-slskr-native-inspector-title]').first().innerText();
        selectedStatus = await page.locator('#slskr-action-status').innerText();
        const rowAction = page.locator('.slskr-native-row-actions button, [data-slskr-native-preview-action]').first();
        if (await rowAction.count()) {
          await rowAction.click();
          toastText = await page.locator('#slskr-toast-region').innerText().catch(() => '');
          confirmationVisible = await page.locator('.slskr-modal-backdrop').isVisible().catch(() => false);
        }
      }
      const layout = await page.evaluate(() => {
        const player = document.querySelector('[data-slskr-player]');
        const main = document.querySelector('.slskr-main');
        const workflow = document.querySelector('.slskr-native-workspace');
        const playerBox = player?.getBoundingClientRect();
        const workflowBox = workflow?.getBoundingClientRect();
        const mainStyle = main ? window.getComputedStyle(main) : null;
        return {
          bodyPaddingBottom: Number.parseFloat(window.getComputedStyle(document.body).paddingBottom || '0'),
          mainPaddingBottom: Number.parseFloat(mainStyle?.paddingBottom || '0'),
          playerHeight: playerBox?.height || 0,
        };
      });

      const routeResult = {
        developerOpen,
        heading,
        inspectorCount,
        layout,
        nativeWorkspaceCount,
        path,
        primaryActionCount,
        confirmationVisible,
        rowCount,
        screenshot: `${slug}-${viewport.name}.png`,
        selectedStatus,
        selectedTitle,
        toastText,
        viewport: viewport.name,
      };
      audit.routes.push(routeResult);

      if (heading !== title) audit.errors.push(`${path} ${viewport.name}: expected heading ${title}, got ${heading}`);
      if (developerOpen) audit.errors.push(`${path} ${viewport.name}: Developer drawer is open by default`);
      if (bodyText.includes('GET /api/v0')) audit.errors.push(`${path} ${viewport.name}: visible raw API text outside Developer drawer`);
      if (primaryActionCount < 1) audit.errors.push(`${path} ${viewport.name}: no primary workflow action`);
      if (nativeWorkspaceCount < 1) audit.errors.push(`${path} ${viewport.name}: missing native workspace`);
      if (inspectorCount < 1) audit.errors.push(`${path} ${viewport.name}: missing inspector/detail surface`);
      if (rowCount < 1) audit.errors.push(`${path} ${viewport.name}: no selectable workflow rows from mocked daemon data`);
      if (rowCount > 0 && (!selectedTitle || selectedTitle === 'Nothing selected')) {
        audit.errors.push(`${path} ${viewport.name}: row selection did not update inspector`);
      }
      if (rowCount > 0 && !selectedStatus.includes('Selected')) {
        audit.errors.push(`${path} ${viewport.name}: row selection did not update action status`);
      }
      if (rowCount > 0 && !toastText.trim() && !selectedStatus.trim() && !confirmationVisible) {
        audit.errors.push(`${path} ${viewport.name}: selected row action did not produce toast/status feedback`);
      }
      if (viewport.name === 'desktop' && layout.mainPaddingBottom < Math.max(72, layout.playerHeight - 4)) {
        audit.errors.push(`${path} ${viewport.name}: main layout does not reserve bottom player space`);
      }
      if (pageErrors.length > 0) audit.errors.push(`${path} ${viewport.name}: browser errors: ${pageErrors.join(' | ')}`);
      await page.close();
    }
  }
} finally {
  await browser.close();
  server.close();
}

await writeFile(join(auditDir, 'audit.json'), `${JSON.stringify(audit, null, 2)}\n`);

if (audit.errors.length > 0) {
  console.error(audit.errors.join('\n'));
  process.exit(1);
}

console.log(`Rust web UI audit passed for ${routes.length} routes across desktop and mobile.`);
