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
const liveBackendUrl = (process.env.SLSKR_REACT_WEB_AUDIT_BACKEND_URL || '').replace(/\/$/u, '');
const liveBackendAuth = process.env.SLSKR_REACT_WEB_AUDIT_AUTH_HEADER || '';
const liveBackendToken = process.env.SLSKR_REACT_WEB_AUDIT_TOKEN || '';
const allowLiveErrors = process.env.SLSKR_REACT_WEB_AUDIT_ALLOW_LIVE_ERRORS === '1';
const requireLiveRows = process.env.SLSKR_REACT_WEB_AUDIT_REQUIRE_ROWS === '1';
const allowedLiveStatusRules = (process.env.SLSKR_REACT_WEB_AUDIT_ALLOWED_LIVE_STATUS || '')
  .split(',')
  .map((rule) => rule.trim().split('|'))
  .filter((rule) => rule.length === 3 && /^\d{3}$/u.test(rule[0]) && rule[1] && rule[2]);
const isAllowedLiveStatus = (status, method, pathName) => allowedLiveStatusRules.some(
  ([allowedStatus, allowedMethod, allowedPath]) =>
    Number(allowedStatus) === status
      && allowedMethod.toUpperCase() === method.toUpperCase()
      && (pathName === allowedPath || pathName.startsWith(`${allowedPath}?`)),
);
const clickActions = liveBackendUrl
  ? process.env.SLSKR_REACT_WEB_AUDIT_CLICK_ACTIONS === '1'
  : process.env.SLSKR_REACT_WEB_AUDIT_CLICK_ACTIONS !== '0';

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
  '/system/mesh',
  '/system/bridge',
  '/system/mediacore',
  '/system/policies',
  '/system/experience',
  '/system/integrations',
  '/system/jobs',
  '/system/automations',
  '/system/source-providers',
  '/system/swarm-analytics',
  '/system/library-health',
  '/system/quarantine-jury',
  '/system/files',
  '/system/data',
  '/system/metrics',
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
const requestedRoutes = process.env.SLSKR_REACT_WEB_AUDIT_ROUTES
  ? new Set(process.env.SLSKR_REACT_WEB_AUDIT_ROUTES.split(',').map((route) => route.trim()))
  : null;
const auditedRoutes = requestedRoutes
  ? routes.filter((route) => requestedRoutes.has(route))
  : routes;
const auditedNavigableRoutes = requestedRoutes
  ? navigableRoutes.filter((route) => requestedRoutes.has(route))
  : navigableRoutes;
const navigationTimeoutMs = 15_000;
const networkIdleTimeoutMs = 3_000;
const auditScenario = process.env.SLSKR_REACT_WEB_AUDIT_SCENARIO || 'success';
const scenarioAttempts = new Map();
const requestedViewports = process.env.SLSKR_REACT_WEB_AUDIT_VIEWPORTS
  ? new Set(process.env.SLSKR_REACT_WEB_AUDIT_VIEWPORTS.split(',').map((viewport) => viewport.trim()))
  : null;
const viewports = [
  { height: 1000, name: 'desktop', width: 1440 },
  { height: 844, name: 'mobile', width: 390 },
].filter((viewport) => !requestedViewports || requestedViewports.has(viewport.name));
const skipScreenshots = process.env.SLSKR_REACT_WEB_AUDIT_SKIP_SCREENSHOTS === '1';
const skipNavigation = process.env.SLSKR_REACT_WEB_AUDIT_SKIP_NAVIGATION === '1';
const browserExecutablePath = process.env.SLSKR_PLAYWRIGHT_EXECUTABLE_PATH || undefined;
const endpointSweep = process.env.SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP
  ? JSON.parse(process.env.SLSKR_REACT_WEB_AUDIT_ENDPOINT_SWEEP)
  : [];

const navigationTestIds = {
  '/searches': 'nav-search',
  '/wishlist': 'nav-wishlist',
  '/downloads': 'nav-downloads',
  '/uploads': 'nav-uploads',
  '/messages': 'nav-messages',
  '/users': 'nav-users',
  '/system': 'nav-system',
  '/discovery-graph': 'nav-discovery-graph',
  '/playlist-intake': 'nav-playlist-intake',
  '/contacts': 'nav-contacts',
  '/solid': 'nav-solid',
  '/collections': 'nav-collections',
  '/sharegroups': 'nav-groups',
  '/shared': 'nav-shared-with-me',
  '/browse': 'nav-browse',
};

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
            message: 'The click-track folder is browseable now.',
            timestamp: '2026-05-05T18:10:00.000Z',
            username: 'audio_lab',
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
        message: 'Thanks, queued the sample file.',
        timestamp: '2026-05-05T18:12:00.000Z',
        username: 'local_operator',
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

const emptyPayload = (value) => {
  if (Array.isArray(value)) return [];
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, emptyPayload(child)]));
  }
  if (typeof value === 'boolean') return false;
  if (typeof value === 'number') return 0;
  if (typeof value === 'string') return '';
  return value;
};

const json = (data, status = 200) => ({
  body: JSON.stringify(auditScenario === 'rendered-loading-and-empty' ? emptyPayload(data) : data),
  contentType: 'application/json',
  status: auditScenario === 'rendered-validation-and-server-error' ? 422 : status,
});

const liveScenarioResponse = async (response, method, pathName) => {
  let status = response.status;
  let body = Buffer.from(await response.arrayBuffer());
  const contentType = response.headers.get('content-type') || '';
  if (auditScenario === 'authorization-reconnect-and-restart') {
    const key = `${method} ${pathName}`;
    const attempt = (scenarioAttempts.get(key) || 0) + 1;
    scenarioAttempts.set(key, attempt);
    if (attempt === 1) {
      status = 401;
      body = Buffer.from(JSON.stringify({ error: 'audit authorization expired' }));
    }
  } else if (contentType.includes('json')) {
    try {
      const parsed = JSON.parse(body.toString('utf8'));
      if (auditScenario === 'rendered-loading-and-empty') {
        body = Buffer.from(JSON.stringify(emptyPayload(parsed)));
      } else if (auditScenario === 'rendered-validation-and-server-error') {
        status = 422;
        body = Buffer.from(JSON.stringify({ error: 'audit validation failure' }));
      }
    } catch {
      if (auditScenario === 'rendered-validation-and-server-error') {
        status = 422;
        body = Buffer.from(JSON.stringify({ error: 'audit validation failure' }));
      }
    }
  }
  return { body, status };
};

const normalizeApiPath = (url) =>
  new URL(url).pathname.replace(/^\/api\/v0/, '').replace(/^\/api/, '');

const fallback = (url, method = 'GET') => {
  const pathname = normalizeApiPath(url);

  if (auditScenario === 'authorization-reconnect-and-restart') {
    const key = `${method} ${pathname}`;
    const attempt = (scenarioAttempts.get(key) || 0) + 1;
    scenarioAttempts.set(key, attempt);
    if (attempt === 1) return json({ error: 'audit authorization expired' }, 401);
  }

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
  if (pathname === '/mesh/peers') {
    return json([
      {
        lastSeqId: 42,
        lastSyncAt: '2026-05-05T18:21:00.000Z',
        username: 'commons_peer',
      },
    ]);
  }
  if (pathname === '/mesh/stats') {
    return json({
      currentSeqId: 42,
      knownMeshPeers: 1,
      isSyncing: false,
      warnings: [],
    });
  }
  if (pathname === '/mesh/transport') {
    return json({
      dht: 0,
      natType: 'Unknown',
      overlay: 1,
    });
  }
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
  if (pathname.match(/^\/users\/[^/]+\/browse\/status$/u)) {
    return json({ isComplete: true, state: 'Completed' });
  }
  if (pathname.startsWith('/users/') && pathname.endsWith('/browse')) {
    return json({
      directories: [
        {
          fileCount: 1,
          files: [{ filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg', size: 153_301 }],
          name: 'open-fixtures',
        },
      ],
      lockedDirectories: [],
      username: 'commons_peer',
    });
  }
  if (pathname.startsWith('/users/')) return json({ username: 'commons_peer' });
  if (pathname === '/contacts') {
    return json([]);
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

const stopStaticServer = async (server) => {
  server.closeAllConnections?.();
  server.closeIdleConnections?.();
  if (!server.listening) return;
  await new Promise((resolve, reject) => {
    server.close((error) => {
      if (error && error.code !== 'ERR_SERVER_NOT_RUNNING') {
        reject(error);
        return;
      }
      resolve();
    });
  });
};

const installMocks = async (page, { activeUser, browseTabs, onApiResponse } = {}) => {
  await page.addInitScript(
    ({ activeUser, appState, browseTabs, liveBackendUrl, options, scenario, searchList, token }) => {
      window.localStorage.setItem('slskr-theme', 'slskr');
      window.sessionStorage.setItem('slskr-token', token || 'audit-token');
      if (activeUser) window.localStorage.setItem('slskr-active-user', activeUser);
      if (browseTabs) {
        window.localStorage.setItem(
          'slskr-browse-tabs',
          JSON.stringify(browseTabs),
        );
      }

      if (liveBackendUrl) {
        const NativeWebSocket = window.WebSocket;
        class LiveWebSocket extends NativeWebSocket {
          constructor(url, protocols) {
            const requested = new URL(url, window.location.origin);
            const backend = new URL(liveBackendUrl);
            const target = new URL(`${requested.pathname}${requested.search}`, backend.origin);
            target.protocol = backend.protocol === 'https:' ? 'wss:' : 'ws:';
            super(target.toString(), protocols);
          }
        }
        LiveWebSocket.CONNECTING = NativeWebSocket.CONNECTING;
        LiveWebSocket.OPEN = NativeWebSocket.OPEN;
        LiveWebSocket.CLOSING = NativeWebSocket.CLOSING;
        LiveWebSocket.CLOSED = NativeWebSocket.CLOSED;
        window.WebSocket = LiveWebSocket;
        return;
      }

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
            if (String(url).includes('/api/events/ws') && scenario === 'authorization-reconnect-and-restart') {
              setTimeout(() => {
                this.readyState = 3;
                this.onclose?.({ code: 1006, reason: 'audit restart' });
                setTimeout(() => {
                  this.readyState = 1;
                  this.onopen?.({});
                }, 25);
              }, 35);
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
    {
      activeUser,
      appState: applicationState,
      browseTabs,
      liveBackendUrl,
      options: applicationOptions,
      searchList: searches,
      scenario: auditScenario,
      token: liveBackendToken,
    },
  );

  await page.route('**/*', async (route) => {
    const url = route.request().url();
    if (!liveBackendUrl && (url.includes('/api/v0/') || url.includes('/api/'))) {
      const mockResponse = fallback(url, route.request().method());
      onApiResponse?.({
        method: route.request().method(),
        path: new URL(url).pathname,
        status: mockResponse.status,
      });
      return route.fulfill(mockResponse);
    }
    if (!liveBackendUrl) return route.continue();

    const request = route.request();
    const requestedUrl = new URL(request.url());
    if (!requestedUrl.pathname.startsWith('/api/')) return route.continue();
    const targetUrl = `${liveBackendUrl}${requestedUrl.pathname}${requestedUrl.search}`;
    const headers = Object.fromEntries(
      Object.entries(request.headers()).filter(
        ([name]) => !['host', 'content-length', 'content-encoding', 'transfer-encoding'].includes(name),
      ),
    );
    if (liveBackendAuth) headers.authorization = liveBackendAuth;
    try {
      const response = await fetch(targetUrl, {
        method: request.method(),
        headers,
        body: ['GET', 'HEAD'].includes(request.method()) ? undefined : request.postDataBuffer() || undefined,
      });
      const scenarioResponse = await liveScenarioResponse(
        response,
        request.method(),
        requestedUrl.pathname,
      );
      const responseRecord = {
        allowed: auditScenario !== 'success'
          || isAllowedLiveStatus(scenarioResponse.status, request.method(), requestedUrl.pathname),
        method: request.method(),
        path: `${requestedUrl.pathname}${requestedUrl.search}`,
        status: scenarioResponse.status,
      };
      onApiResponse?.(responseRecord);
      const responseHeaders = Object.fromEntries(
        [...response.headers].filter(
          ([name]) => !['content-encoding', 'content-length', 'transfer-encoding'].includes(name),
        ),
      );
      return route.fulfill({
        body: scenarioResponse.body,
        headers: responseHeaders,
        status: scenarioResponse.status,
      });
    } catch (error) {
      onApiResponse?.({
        method: request.method(),
        path: `${requestedUrl.pathname}${requestedUrl.search}`,
        status: 599,
        allowed: false,
      });
      if (error?.message) {
        onApiResponse?.({
          method: 'AUDIT_ERROR',
          path: `${requestedUrl.pathname}${requestedUrl.search}`,
          status: 599,
          allowed: false,
          error: error.message,
        });
      }
      if (allowLiveErrors) {
        return route.fulfill({
          body: JSON.stringify({ error: error?.message || 'live backend proxy failed' }),
          contentType: 'application/json',
          status: 599,
        });
      }
      throw error;
    }
  });
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
    const selectors = ['.ui.menu a.item', '.ui.button', 'button', 'input', '.ui.card'];
    const elements = selectors
      .flatMap((selector) => Array.from(document.querySelectorAll(selector)))
      .filter((element) => {
        if (element.closest('table, .ui.table')) return false;
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
          if (elements[i].tagName === 'SELECT' || elements[j].tagName === 'SELECT') {
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
const browser = await chromium.launch({
  executablePath: browserExecutablePath,
  headless: process.env.HEADLESS !== 'false',
});
const audit = {
  apiResponses: [],
  allowedLiveStatus: allowedLiveStatusRules.map((rule) => rule.join('|')),
  allowLiveErrors,
  baseUrl,
  evidenceMode: liveBackendUrl ? 'live' : 'mock',
  errors: [],
  generatedAt: new Date().toISOString(),
  routes: [],
  scenario: auditScenario,
};

try {
  if (!skipNavigation) {
    const navigationContext = await browser.newContext({
      serviceWorkers: 'block',
      viewport: { width: 1440, height: 1000 },
    });
    const page = await navigationContext.newPage();
    await installMocks(page);

    for (const target of auditedNavigableRoutes) {
      await page.goto(`${baseUrl}/searches`, {
        timeout: navigationTimeoutMs,
        waitUntil: 'domcontentloaded',
      });
      await page
        .waitForLoadState('networkidle', { timeout: networkIdleTimeoutMs })
        .catch(() => {});
      const moreMenu = page.locator('[data-testid="nav-more"]:visible').first();
      if (await moreMenu.count()) await moreMenu.click();
      const link = page
        .locator(`[data-testid="${navigationTestIds[target] || ''}"]`)
        .first();
      if ((await link.count()) === 0) {
        audit.errors.push(`navigation link missing: ${target}`);
        continue;
      }
      await link.evaluate((element) => element.click());
      await page
        .waitForLoadState('networkidle', { timeout: networkIdleTimeoutMs })
        .catch(() => {});
      if (!page.url().includes(target)) {
        audit.errors.push(`navigation link did not reach ${target}; landed on ${page.url()}`);
      }
    }

    await page.close();
    await navigationContext.close();
  }

  for (const route of auditedRoutes) {
    for (const viewport of viewports) {
      const routeContext = await browser.newContext({ serviceWorkers: 'block', viewport });
      const routePage = await routeContext.newPage();
      const pageErrors = [];
      const apiResponses = [];
      routePage.on('pageerror', (error) => pageErrors.push(error.stack || error.message));
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
      await installMocks(routePage, {
        activeUser: liveBackendUrl
          ? process.env.SLSKR_REACT_WEB_AUDIT_ACTIVE_USER || undefined
          : route === '/users'
            ? 'commons_peer'
            : undefined,
        browseTabs: liveBackendUrl
          ? undefined
          : route === '/browse'
            ? {
                tabCounter: 1,
                tabs: [{ key: 'tab-1', label: 'commons_peer', username: 'commons_peer' }],
              }
            : undefined,
        onApiResponse: (response) => {
          apiResponses.push(response);
          if (liveBackendUrl) audit.apiResponses.push({ route, viewport: viewport.name, ...response });
          if (liveBackendUrl && response.status >= 400 && !response.allowed && !allowLiveErrors) {
            audit.errors.push(
              `${route} ${viewport.name}: live API ${response.method} ${response.path} returned ${response.status}`,
            );
          }
        },
      });

      let response;
      try {
        response = await routePage.goto(`${baseUrl}${route}`, {
          timeout: navigationTimeoutMs,
          waitUntil: 'domcontentloaded',
        });
      } catch (error) {
        audit.errors.push(
          `${route} ${viewport.name}: navigation failed: ${error?.message || error}`,
        );
        await routePage.close();
        await routeContext.close();
        continue;
      }
      await routePage
        .waitForLoadState('networkidle', { timeout: networkIdleTimeoutMs })
        .catch(() => {});
      await routePage.waitForTimeout(auditScenario === 'success' ? 250 : 125);
      if (
        endpointSweep.length > 0
        && route === auditedRoutes[0]
        && viewport.name === viewports[0]?.name
      ) {
        await routePage.evaluate(async (endpoints) => {
          for (const endpoint of endpoints) {
            try {
              await fetch(endpoint.url, {
                body: endpoint.method === 'GET' || endpoint.method === 'HEAD' ? undefined : '{}',
                headers: endpoint.method === 'GET' || endpoint.method === 'HEAD'
                  ? undefined
                  : { 'content-type': 'application/json' },
                method: endpoint.method,
              });
            } catch {
              // The response listener records the server result when one exists.
            }
          }
        }, endpointSweep);
        await routePage.waitForTimeout(125);
      }

      const bodyText = await routePage.locator('body').innerText().catch(() => '');
      const rootChildCount = await routePage.locator('#root > *').count();
      const visibleButtonCount = await routePage.locator('button:visible, .ui.button:visible').count();
      const visibleInputCount = await routePage.locator('input:visible, textarea:visible').count();
      const internalHrefs = await visibleInternalHrefs(routePage);
      const overlaps = await assertNoOverlap(routePage);
      const screenshot = skipScreenshots ? null : `${slugFor(route)}-${viewport.name}.png`;
      if (screenshot) {
        await routePage.screenshot({
          fullPage: false,
          path: path.join(outputDir, screenshot),
        });
      }

      const result = {
        bodyLength: bodyText.length,
        apiResponses,
        internalHrefs: [...new Set(internalHrefs)].sort(),
        overlaps,
        responseStatus: response?.status(),
        rootChildCount,
        route,
        scenario: auditScenario,
        screenshot,
        visibleButtonCount,
        visibleInputCount,
        viewport: viewport.name,
      };
      audit.routes.push(result);

      if (auditScenario === 'success' && response?.status() !== 200) {
        audit.errors.push(`${route} ${viewport.name}: HTTP ${response?.status()}`);
      }
      if (rootChildCount < 1) audit.errors.push(`${route} ${viewport.name}: React root did not mount`);
      if (bodyText.length < 100) audit.errors.push(`${route} ${viewport.name}: page looks blank`);
      if (/not found|cannot get|404/iu.test(bodyText)) audit.errors.push(`${route} ${viewport.name}: visible 404 text`);
      if (bodyText.includes('Rust Web')) audit.errors.push(`${route} ${viewport.name}: Rust migration UI leaked into React audit`);
      if (visibleButtonCount + visibleInputCount < 1 && route !== '/') {
        audit.errors.push(`${route} ${viewport.name}: no visible controls`);
      }
      if (liveBackendUrl && requireLiveRows && apiResponses.length > 0 && visibleButtonCount + visibleInputCount < 1) {
        audit.errors.push(`${route} ${viewport.name}: live backend rendered no actionable controls`);
      }
      if (overlaps.length > 0) {
        audit.errors.push(`${route} ${viewport.name}: overlapping controls ${JSON.stringify(overlaps)}`);
      }
      for (const href of internalHrefs) {
        if (!routes.includes(href) && !href.startsWith('/searches/')) {
          audit.errors.push(`${route} ${viewport.name}: untracked internal link ${href}`);
        }
      }
      const unexpectedPageErrors = pageErrors.filter((error) =>
        !liveBackendUrl
        || !allowedLiveStatusRules.some(([status]) => error.includes(`status code ${status}`)),
      );
      if (unexpectedPageErrors.length > 0 && auditScenario === 'success') {
        audit.errors.push(`${route} ${viewport.name}: browser errors: ${unexpectedPageErrors.join(' | ')}`);
      }

      if (auditScenario === 'success' && clickActions) {
        const clickTargets = routePage
          .locator('button:visible:not([disabled]), .ui.button:visible:not(.disabled)')
          .filter({ hasNotText: /delete|remove|clear all|disconnect|logout/iu });
        const clickCount = Math.min(await clickTargets.count(), 12);
        for (let index = 0; index < clickCount; index += 1) {
          await clickTargets.nth(index).click({ timeout: 1000 }).catch(() => {});
          await routePage.waitForTimeout(50);
        }
      }
      await routePage.close();
      await routeContext.close();
    }
  }
} finally {
  await browser.close();
  await stopStaticServer(server);
}

await fs.writeFile(path.join(outputDir, 'audit.json'), `${JSON.stringify(audit, null, 2)}\n`);

if (audit.errors.length > 0) {
  console.error(audit.errors.join('\n'));
  process.exit(1);
}

console.log(
  `React Web UI audit passed for ${auditedRoutes.length} routes across ${viewports.length} viewport(s), scenario=${auditScenario}.`,
);
