import { chromium } from '@playwright/test';
import { createServer } from 'node:http';
import fs from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(webRoot, '..');
const buildDir = path.resolve(webRoot, 'build');
const outputDir = path.resolve(
  repoRoot,
  process.env.SLSKR_REACT_WEB_AUDIT_DIR || 'target/react-webui-audit',
);

const routes = [
  '/',
  '/searches',
  '/searches/commons-example-sound',
  '/discovery-graph',
  '/playlist-intake',
  '/wishlist',
  '/downloads',
  '/uploads',
  '/messages',
  '/chat',
  '/rooms',
  '/users',
  '/contacts',
  '/solid',
  '/collections',
  '/sharegroups',
  '/shared',
  '/browse',
  '/system',
  '/system/info',
  '/system/options',
  '/system/shares',
  '/system/logs',
  '/system/events',
  '/system/network',
  '/system/security',
];

const navigableRoutes = [
  '/searches',
  '/discovery-graph',
  '/playlist-intake',
  '/wishlist',
  '/downloads',
  '/uploads',
  '/messages',
  '/users',
  '/contacts',
  '/solid',
  '/collections',
  '/sharegroups',
  '/shared',
  '/browse',
  '/system',
];

const searches = [
  {
    endedAt: '2026-05-05T18:20:00.000Z',
    fileCount: 18,
    id: 'commons-example-sound',
    lockedFileCount: 0,
    responseCount: 2,
    searchText: 'Example sound file Ogg Vorbis',
    startedAt: '2026-05-05T18:18:00.000Z',
    state: 'Completed',
  },
  {
    endedAt: null,
    fileCount: 4,
    id: 'commons-click-track',
    lockedFileCount: 1,
    responseCount: 1,
    searchText: 'Audacity click track eight seconds',
    startedAt: '2026-05-05T18:22:00.000Z',
    state: 'InProgress',
  },
];

const searchResponses = [
  {
    averageSpeed: 512_000,
    files: [
      {
        bitRate: 192,
        filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg',
        length: 84,
        path: 'open-fixtures/Example_sound_file_in_Ogg_Vorbis_format.ogg',
        size: 153_301,
      },
    ],
    hasFreeUploadSlot: true,
    isLocked: false,
    queueLength: 0,
    token: 1,
    username: 'commons_peer',
  },
  {
    averageSpeed: 96_000,
    files: [
      {
        bitRate: 1411,
        filename: 'Example_sound_file_lossless.flac',
        length: 84,
        path: 'open-fixtures/Example_sound_file_lossless.flac',
        size: 2_400_000,
      },
    ],
    hasFreeUploadSlot: false,
    isLocked: false,
    queueLength: 2,
    token: 1,
    username: 'audio_lab',
  },
];

const transfers = [
  {
    username: 'commons_peer',
    directories: [
      {
        directory: 'open-fixtures',
        files: [
          {
            averageSpeed: 96_000,
            bytesRemaining: 0,
            bytesTransferred: 153_301,
            direction: 'Download',
            elapsedTime: '00:00:02',
            filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg',
            id: 'transfer-commons-example-sound',
            percentComplete: 100,
            placeInQueue: 0,
            remainingTime: '00:00:00',
            size: 153_301,
            startOffset: 0,
            state: 'Completed, Succeeded',
            username: 'commons_peer',
          },
          {
            averageSpeed: 64_000,
            bytesRemaining: 3_820,
            bytesTransferred: 3_820,
            direction: 'Download',
            elapsedTime: '00:00:01',
            filename: 'Audacity_click_track_one_per_second.ogg',
            id: 'transfer-commons-click-track',
            percentComplete: 50,
            placeInQueue: 1,
            remainingTime: '00:00:01',
            size: 7_640,
            startOffset: 0,
            state: 'InProgress',
            username: 'commons_peer',
          },
        ],
      },
    ],
  },
];

const conversations = [
  {
    id: 'conversation-audio-lab',
    messages: [
      {
        direction: 'Incoming',
        id: 'message-1',
        isAcknowledged: false,
        sentAt: '2026-05-05T18:10:00.000Z',
        text: 'The click-track folder is browseable now.',
      },
    ],
    username: 'audio_lab',
  },
  {
    id: 'conversation-commons-peer',
    messages: [
      {
        direction: 'Outgoing',
        id: 'message-2',
        isAcknowledged: true,
        sentAt: '2026-05-05T18:12:00.000Z',
        text: 'Thanks, queued the sample file.',
      },
    ],
    username: 'commons_peer',
  },
];

const applicationState = {
  connectionWatchdog: {
    lastCheckAt: '2026-05-05T18:24:00.000Z',
    status: 'healthy',
  },
  relay: { mode: 'direct' },
  server: {
    address: 'vps.slsknet.org:2271',
    isConnected: true,
    isConnecting: false,
    username: 'local_operator',
  },
  shares: {
    directoryCount: 4,
    fileCount: 128,
    scanPending: false,
    scannedAt: '2026-05-05T18:00:00.000Z',
    size: 3_400_000_000,
  },
  transfers: {
    down: 64_000,
    up: 12_000,
  },
  user: {
    username: 'local_operator',
  },
  version: {
    current: '0.0.0',
    isUpdateAvailable: false,
    latest: '0.0.0',
  },
  vpn: {
    isReady: false,
  },
};

const applicationOptions = {
  directories: {
    downloads: '/srv/slskr/downloads',
    incomplete: '/srv/slskr/incomplete',
  },
  shares: {
    directories: ['/srv/media/open-fixtures'],
  },
};

const contentTypeFor = (filePath) => {
  switch (path.extname(filePath)) {
    case '.css':
      return 'text/css; charset=utf-8';
    case '.html':
      return 'text/html; charset=utf-8';
    case '.ico':
      return 'image/x-icon';
    case '.js':
      return 'text/javascript; charset=utf-8';
    case '.json':
    case '.webmanifest':
      return 'application/json; charset=utf-8';
    case '.png':
      return 'image/png';
    case '.svg':
      return 'image/svg+xml';
    case '.woff':
      return 'font/woff';
    case '.woff2':
      return 'font/woff2';
    default:
      return 'application/octet-stream';
  }
};

const json = (data, status = 200) => ({
  body: JSON.stringify(data),
  contentType: 'application/json',
  status,
});

const normalizeApiPath = (url) =>
  new URL(url).pathname.replace(/^\/api\/v0/, '').replace(/^\/api/, '');

const fallback = (url, method = 'GET') => {
  const pathname = normalizeApiPath(url);

  if (method !== 'GET') {
    if (pathname === '/searches') return json(searches[0], 201);
    return json({ accepted: true, ok: true });
  }

  if (pathname === '/session/enabled') return json(true);
  if (pathname === '/session') return json({ username: 'local_operator' });
  if (pathname === '/application') return json(applicationState);
  if (pathname === '/options') return json(applicationOptions);
  if (pathname === '/server') return json(applicationState.server);
  if (pathname === '/health') return json({ service: 'slskr', status: 'ok' });
  if (pathname === '/application/version/latest') return json(applicationState.version);
  if (pathname === '/capabilities') {
    return json({
      api: ['health', 'events', 'metrics', 'telemetry'],
      feature: { scenePodBridge: false },
      network: ['server-session', 'peer-messaging', 'file-transfer'],
      storage: ['share-index', 'transfer-state'],
    });
  }
  if (pathname.match(/^\/searches\/[^/]+\/responses$/u)) return json(searchResponses);
  if (pathname.startsWith('/searches/')) return json(searches[0]);
  if (pathname === '/searches') return json(searches);
  if (pathname === '/transfers/downloads') return json(transfers);
  if (pathname === '/transfers/uploads') return json(transfers);
  if (pathname === '/transfers/speeds') return json({ download: 64_000, upload: 12_000 });
  if (pathname === '/transfers/downloads/accelerated') return json({ enabled: true });
  if (pathname === '/transfers/downloads/user-stats') {
    return json({
      audio_lab: { failedDownloads: 0, successfulDownloads: 3 },
      commons_peer: { failedDownloads: 0, successfulDownloads: 12 },
    });
  }
  if (pathname === '/rooms/available') return json(['ambient', 'field-recordings', 'netlabel']);
  if (pathname === '/rooms/joined') return json(['ambient', 'netlabel']);
  if (pathname.includes('/rooms/joined/') && pathname.endsWith('/messages')) {
    return json([
      {
        body: 'New open fixture set is mirrored.',
        direction: 'In',
        timestamp: '2026-05-05T18:16:00.000Z',
        username: 'commons_peer',
      },
    ]);
  }
  if (pathname.includes('/rooms/joined/') && pathname.endsWith('/users')) {
    return json(['local_operator', 'commons_peer', 'audio_lab']);
  }
  if (pathname === '/conversations') return json(conversations);
  if (pathname.startsWith('/conversations/')) return json(conversations[0]);
  if (pathname === '/wishlist') {
    return json([
      {
        createdAt: '2026-05-05T17:55:00.000Z',
        id: 'wishlist-1',
        lastSearchId: 'commons-example-sound',
        searchText: 'Example sound file Ogg Vorbis',
      },
    ]);
  }
  if (pathname === '/users') {
    return json([
      { files: 128, privileged: true, status: 'Online', username: 'commons_peer' },
      { files: 42, privileged: false, status: 'Away', username: 'audio_lab' },
    ]);
  }
  if (pathname === '/users/notes') return json([]);
  if (pathname.startsWith('/users/notes/')) return json({});
  if (pathname.startsWith('/users/') && pathname.endsWith('/browse')) {
    return json({
      directories: [
        {
          files: [{ filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg', size: 153_301 }],
          name: 'open-fixtures',
        },
      ],
      username: 'commons_peer',
    });
  }
  if (pathname.startsWith('/users/')) return json({ username: 'commons_peer' });
  if (pathname === '/contacts') {
    return json([
      { group: 'Friends', nickname: 'Commons Peer', status: 'Online', username: 'commons_peer' },
    ]);
  }
  if (pathname === '/shares') {
    return json({
      directories: applicationOptions.shares.directories,
      fileCount: 128,
      size: 3_400_000_000,
    });
  }
  if (pathname === '/shares/contents') {
    return json([
      { filename: 'commons-example-sound.ogg', size: 153_301 },
      { filename: 'commons-click-track.ogg', size: 7_640 },
    ]);
  }
      if (pathname === '/collections') {
        return json([
          { id: 'collection-open-fixtures', itemCount: 2, title: 'Open fixture recordings' },
        ]);
      }
  if (pathname.startsWith('/collections/')) return json([]);
      if (pathname === '/sharegroups') {
        return json([{ id: 'friends', memberCount: 2, name: 'Friends', permission: 'read' }]);
      }
  if (pathname.startsWith('/sharegroups/')) return json([]);
  if (pathname === '/shared') {
    return json([{ id: 'grant-1', owner: 'commons_peer', title: 'Open fixture grant' }]);
  }
  if (pathname === '/events') return json([]);
  if (pathname.includes('/logs')) return json([{ level: 'info', message: 'audit log' }]);
  if (pathname.includes('/metrics') || pathname.includes('/telemetry')) return json({});
  if (pathname.includes('/status')) return json({ enabled: false, status: 'disabled' });

  return json([]);
};

const startStaticServer = async () => {
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url || '/', 'http://127.0.0.1');
      const decodedPath = decodeURIComponent(url.pathname);
      let filePath = path.join(buildDir, decodedPath);
      if (decodedPath === '/' || !existsSync(filePath)) {
        filePath = path.join(buildDir, 'index.html');
      }
      let body = await fs.readFile(filePath);
      if (filePath.endsWith('index.html')) {
        body = Buffer.from(
          body.toString('utf8').replace('<head>', '<head><base href="/" />'),
        );
      }
      response.writeHead(200, { 'content-type': contentTypeFor(filePath) });
      response.end(body);
    } catch (error) {
      response.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
      response.end(error?.message || 'static server error');
    }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return server;
};

const installMocks = async (page) => {
  await page.addInitScript(
    ({ appState, options, searchList }) => {
      window.localStorage.setItem('slskr-theme', 'slskr');
      window.sessionStorage.setItem('slskr-token', 'audit-token');

      class FakeWebSocket {
        constructor(url) {
          this.url = url;
          this.readyState = 0;
          setTimeout(() => {
            this.readyState = 1;
            this.onopen?.({});
            if (String(url).includes('/api/events/ws')) {
              this.onmessage?.({
                data: JSON.stringify({
                  data: searchList,
                  topic: 'search',
                  type: 'search.list',
                }),
              });
              this.onmessage?.({
                data: JSON.stringify({
                  data: appState,
                  topic: 'application',
                  type: 'session.updated',
                }),
              });
            }
          }, 20);
        }

        close() {
          this.readyState = 3;
          this.onclose?.({});
        }

        send() {}
      }

      FakeWebSocket.CONNECTING = 0;
      FakeWebSocket.OPEN = 1;
      FakeWebSocket.CLOSING = 2;
      FakeWebSocket.CLOSED = 3;
      window.WebSocket = FakeWebSocket;
    },
    { appState: applicationState, options: applicationOptions, searchList: searches },
  );

  await page.route('**/api/v0/**', (route) =>
    route.fulfill(fallback(route.request().url(), route.request().method())),
  );
  await page.route('**/api/**', (route) =>
    route.fulfill(fallback(route.request().url(), route.request().method())),
  );
};

const slugFor = (route) =>
  route === '/' ? 'root' : route.replace(/^\//u, '').replace(/[^\w-]+/gu, '-');

const visibleInternalHrefs = async (page) =>
  page
    .locator('a[href]')
    .evaluateAll((anchors) =>
      anchors
        .filter((anchor) => {
          const style = window.getComputedStyle(anchor);
          const box = anchor.getBoundingClientRect();
          return style.visibility !== 'hidden' && style.display !== 'none' && box.width > 0 && box.height > 0;
        })
        .map((anchor) => anchor.getAttribute('href'))
        .filter((href) => href && href.startsWith('/')),
    );

const assertNoOverlap = async (page) =>
  page.evaluate(() => {
    const selectors = ['.ui.menu a.item', '.ui.button', 'button', 'input', '.ui.card', '.ui.table'];
    const elements = selectors
      .flatMap((selector) => Array.from(document.querySelectorAll(selector)))
      .filter((element) => {
        const box = element.getBoundingClientRect();
        const style = window.getComputedStyle(element);
        return box.width > 0 && box.height > 0 && style.visibility !== 'hidden' && style.display !== 'none';
      });

    const overlaps = [];
    for (let i = 0; i < elements.length; i += 1) {
      const a = elements[i].getBoundingClientRect();
      for (let j = i + 1; j < elements.length; j += 1) {
        const b = elements[j].getBoundingClientRect();
        const intersectionWidth = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
        const intersectionHeight = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
        if (intersectionWidth > 8 && intersectionHeight > 8) {
          if (elements[i].contains(elements[j]) || elements[j].contains(elements[i])) {
            continue;
          }
          const aText = elements[i].textContent?.trim() || '';
          const bText = elements[j].textContent?.trim() || '';
          if (
            !aText ||
            !bText ||
            aText === bText ||
            aText === 'BUTTON' ||
            bText === 'BUTTON'
          ) {
            continue;
          }
          const aArea = a.width * a.height;
          const bArea = b.width * b.height;
          const intersectionArea = intersectionWidth * intersectionHeight;
          if (intersectionArea / Math.min(aArea, bArea) > 0.6) {
            overlaps.push({
              a: elements[i].textContent?.trim().slice(0, 80) || elements[i].tagName,
              b: elements[j].textContent?.trim().slice(0, 80) || elements[j].tagName,
            });
          }
        }
      }
    }
    return overlaps.slice(0, 5);
  });

if (!existsSync(path.join(buildDir, 'index.html'))) {
  throw new Error('web/build/index.html is missing; run npm --prefix web run build first.');
}

await fs.mkdir(outputDir, { recursive: true });

const server = await startStaticServer();
const { port } = server.address();
const baseUrl = `http://127.0.0.1:${port}`;
const browser = await chromium.launch({ headless: process.env.HEADLESS !== 'false' });
const audit = {
  baseUrl,
  errors: [],
  generatedAt: new Date().toISOString(),
  routes: [],
};

try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  await installMocks(page);

  for (const target of navigableRoutes) {
    await page.goto(`${baseUrl}/searches`, { waitUntil: 'networkidle' });
    const link = page.locator(`a[href="${target}"]`).first();
    if ((await link.count()) === 0) {
      audit.errors.push(`navigation link missing: ${target}`);
      continue;
    }
    await link.click();
    await page.waitForLoadState('networkidle').catch(() => {});
    if (!page.url().includes(target)) {
      audit.errors.push(`navigation link did not reach ${target}; landed on ${page.url()}`);
    }
  }

  await page.close();

  for (const route of routes) {
    for (const viewport of [
      { height: 1000, name: 'desktop', width: 1440 },
      { height: 844, name: 'mobile', width: 390 },
    ]) {
      const routePage = await browser.newPage({ viewport });
      await installMocks(routePage);
      const pageErrors = [];
      routePage.on('pageerror', (error) => pageErrors.push(error.message));
      routePage.on('console', (message) => {
        const text = message.text();
        if (
          message.type() === 'error' &&
          !text.includes('Failed to load resource') &&
          !text.includes('WebSocket connection')
        ) {
          pageErrors.push(text);
        }
      });

      const response = await routePage.goto(`${baseUrl}${route}`, {
        waitUntil: 'networkidle',
      });
      await routePage.waitForTimeout(250);

      const bodyText = await routePage.locator('body').innerText().catch(() => '');
      const rootChildCount = await routePage.locator('#root > *').count();
      const visibleButtonCount = await routePage.locator('button:visible, .ui.button:visible').count();
      const visibleInputCount = await routePage.locator('input:visible, textarea:visible').count();
      const internalHrefs = await visibleInternalHrefs(routePage);
      const overlaps = await assertNoOverlap(routePage);
      const screenshot = `${slugFor(route)}-${viewport.name}.png`;
      await routePage.screenshot({
        fullPage: false,
        path: path.join(outputDir, screenshot),
      });

      const result = {
        bodyLength: bodyText.length,
        internalHrefs: [...new Set(internalHrefs)].sort(),
        overlaps,
        responseStatus: response?.status(),
        rootChildCount,
        route,
        screenshot,
        visibleButtonCount,
        visibleInputCount,
        viewport: viewport.name,
      };
      audit.routes.push(result);

      if (response?.status() !== 200) audit.errors.push(`${route} ${viewport.name}: HTTP ${response?.status()}`);
      if (rootChildCount < 1) audit.errors.push(`${route} ${viewport.name}: React root did not mount`);
      if (bodyText.length < 100) audit.errors.push(`${route} ${viewport.name}: page looks blank`);
      if (/not found|cannot get|404/iu.test(bodyText)) audit.errors.push(`${route} ${viewport.name}: visible 404 text`);
      if (bodyText.includes('Rust Web')) audit.errors.push(`${route} ${viewport.name}: Rust migration UI leaked into React audit`);
      if (visibleButtonCount + visibleInputCount < 1 && route !== '/') {
        audit.errors.push(`${route} ${viewport.name}: no visible controls`);
      }
      if (overlaps.length > 0) {
        audit.errors.push(`${route} ${viewport.name}: overlapping controls ${JSON.stringify(overlaps)}`);
      }
      for (const href of internalHrefs) {
        if (!routes.includes(href) && !href.startsWith('/searches/')) {
          audit.errors.push(`${route} ${viewport.name}: untracked internal link ${href}`);
        }
      }
      if (pageErrors.length > 0) {
        audit.errors.push(`${route} ${viewport.name}: browser errors: ${pageErrors.join(' | ')}`);
      }

      const clickTargets = routePage
        .locator('button:visible:not([disabled]), .ui.button:visible:not(.disabled)')
        .filter({ hasNotText: /delete|remove|clear all|disconnect|logout/iu });
      const clickCount = Math.min(await clickTargets.count(), 3);
      for (let index = 0; index < clickCount; index += 1) {
        await clickTargets.nth(index).click({ timeout: 1000 }).catch(() => {});
        await routePage.waitForTimeout(50);
      }
      await routePage.close();
    }
  }
} finally {
  await browser.close();
  server.close();
}

await fs.writeFile(path.join(outputDir, 'audit.json'), `${JSON.stringify(audit, null, 2)}\n`);

if (audit.errors.length > 0) {
  console.error(audit.errors.join('\n'));
  process.exit(1);
}

console.log(`React Web UI audit passed for ${routes.length} routes across desktop and mobile.`);
