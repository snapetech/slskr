#!/usr/bin/env node

/**
 * Capture a live, side-by-side UI workflow comparison.
 *
 * This runner intentionally accepts already-running HTTP backends and static
 * UI roots. Keeping process orchestration outside the browser runner makes it
 * possible to use frozen, prebuilt target daemons and keeps the browser's
 * memory budget independently guarded by with-process-memory-guard.sh.
 */

import { createServer } from 'node:http';
import { existsSync, createReadStream } from 'node:fs';
import { access, mkdir, readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('../web/node_modules/playwright');

const outputPath = resolve(
  process.env.SLSKR_TARGET_UI_COMPARISON_OUTPUT ||
    'target/frozen-target-ui-comparison/audit.json',
);
const settleMs = Math.min(
  2000,
  Math.max(100, Number.parseInt(process.env.SLSKR_TARGET_UI_COMPARISON_SETTLE_MS || '500', 10) || 500),
);
const apiQuietMs = Math.min(
  1000,
  Math.max(100, Number.parseInt(process.env.SLSKR_TARGET_UI_COMPARISON_API_QUIET_MS || '250', 10) || 250),
);
const apiCaptureMaxMs = Math.min(
  10000,
  Math.max(
    settleMs,
    Number.parseInt(process.env.SLSKR_TARGET_UI_COMPARISON_API_MAX_WAIT_MS || '5000', 10) || 5000,
  ),
);
const browserExecutablePath =
  process.env.SLSKR_PLAYWRIGHT_EXECUTABLE_PATH ||
  ['/usr/bin/chromium', '/usr/bin/chromium-browser', '/usr/bin/google-chrome'].find(
    (candidate) => existsSync(candidate),
  );
const requirePass = process.env.SLSKR_TARGET_UI_COMPARISON_REQUIRE_PASS !== '0';
const workflows = [
  {
    id: 'search',
    routes: {
      slskd: '/searches',
      slskdn: '/searches',
      'replacement-slskd': '/searches',
      'replacement-slskdn': '/searches',
    },
  },
  {
    id: 'browse',
    routes: {
      slskd: '/browse',
      slskdn: '/browse',
      'replacement-slskd': '/browse',
      'replacement-slskdn': '/browse',
    },
  },
  {
    id: 'transfers',
    routes: {
      slskd: '/downloads',
      slskdn: '/downloads',
      'replacement-slskd': '/downloads',
      'replacement-slskdn': '/downloads',
    },
  },
  {
    id: 'messages',
    routes: {
      slskd: '/chat',
      slskdn: '/messages',
      'replacement-slskd': '/chat',
      'replacement-slskdn': '/messages',
    },
  },
  {
    id: 'rooms',
    routes: {
      slskd: '/rooms',
      slskdn: '/rooms',
      'replacement-slskd': '/rooms',
      'replacement-slskdn': '/rooms',
    },
  },
  {
    id: 'shares',
    routes: {
      slskd: '/system/shares',
      slskdn: '/system/shares',
      'replacement-slskd': '/system/shares',
      'replacement-slskdn': '/system/shares',
    },
  },
  {
    id: 'settings',
    routes: {
      slskd: '/system/options',
      slskdn: '/system/options',
      'replacement-slskd': '/system/options',
      'replacement-slskdn': '/system/options',
    },
  },
  {
    id: 'player',
    routes: {
      slskd: '/dashboard',
      slskdn: '/searches',
      'replacement-slskd': '/dashboard',
      'replacement-slskdn': '/searches',
    },
  },
  {
    id: 'mesh',
    routes: {
      slskd: '/system',
      slskdn: '/system/mesh',
      'replacement-slskd': '/system',
      'replacement-slskdn': '/system/mesh',
    },
  },
];

const surfaces = {
  slskd: {
    backendUrl: process.env.SLSKR_TARGET_SLSKD_BACKEND_URL,
    sameOrigin: process.env.SLSKR_TARGET_SLSKD_UI_SAME_ORIGIN === '1',
    uiRoot: process.env.SLSKR_TARGET_SLSKD_UI_ROOT,
  },
  slskdn: {
    backendUrl: process.env.SLSKR_TARGET_SLSKDN_BACKEND_URL,
    sameOrigin: process.env.SLSKR_TARGET_SLSKDN_UI_SAME_ORIGIN === '1',
    uiRoot: process.env.SLSKR_TARGET_SLSKDN_UI_ROOT,
  },
  'replacement-slskd': {
    backendUrl: process.env.SLSKR_REPLACEMENT_SLSKD_BACKEND_URL,
    sameOrigin: process.env.SLSKR_REPLACEMENT_SLSKD_UI_SAME_ORIGIN === '1',
    uiRoot: process.env.SLSKR_REPLACEMENT_SLSKD_UI_ROOT,
    profile: 'slskd',
    runtimeProfile: 'legacy',
  },
  'replacement-slskdn': {
    backendUrl: process.env.SLSKR_REPLACEMENT_SLSKDN_BACKEND_URL,
    sameOrigin: process.env.SLSKR_REPLACEMENT_SLSKDN_UI_SAME_ORIGIN === '1',
    uiRoot: process.env.SLSKR_REPLACEMENT_SLSKDN_UI_ROOT,
    profile: 'slskdn',
    runtimeProfile: 'native',
  },
};

const contentTypes = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.eot', 'application/vnd.ms-fontobject'],
  ['.gif', 'image/gif'],
  ['.html', 'text/html; charset=utf-8'],
  ['.ico', 'image/x-icon'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.json', 'application/json; charset=utf-8'],
  ['.png', 'image/png'],
  ['.svg', 'image/svg+xml'],
  ['.ttf', 'font/ttf'],
  ['.wasm', 'application/wasm'],
  ['.woff', 'font/woff'],
  ['.woff2', 'font/woff2'],
]);

const requiredEnvironment = [
  'SLSKR_TARGET_SLSKD_BACKEND_URL',
  'SLSKR_TARGET_SLSKD_UI_ROOT',
  'SLSKR_TARGET_SLSKDN_BACKEND_URL',
  'SLSKR_TARGET_SLSKDN_UI_ROOT',
  'SLSKR_REPLACEMENT_SLSKD_BACKEND_URL',
  'SLSKR_REPLACEMENT_SLSKD_UI_ROOT',
  'SLSKR_REPLACEMENT_SLSKDN_BACKEND_URL',
  'SLSKR_REPLACEMENT_SLSKDN_UI_ROOT',
];

const fail = (message) => {
  throw new Error(message);
};

const assertDirectory = async (path, label) => {
  if (!path) fail(`${label} is required`);
  let details;
  try {
    details = await stat(path);
  } catch (error) {
    fail(`${label} does not exist: ${path} (${error.message})`);
  }
  if (!details.isDirectory()) fail(`${label} is not a directory: ${path}`);
  await access(join(path, 'index.html'));
};

const assertBackend = (url, label) => {
  if (!url) fail(`${label} is required`);
  let parsed;
  try {
    parsed = new URL(url);
  } catch (error) {
    fail(`${label} is not a URL: ${url} (${error.message})`);
  }
  if (!['http:', 'https:'].includes(parsed.protocol)) {
    fail(`${label} must use http or https: ${url}`);
  }
  return url.replace(/\/$/u, '');
};

const safePath = (root, requestPath) => {
  const pathname = decodeURIComponent(requestPath.split('?')[0] || '/');
  const assetMarkers = ['/assets/', '/static/'];
  const assetMarker = assetMarkers
    .map((marker) => pathname.indexOf(marker))
    .filter((index) => index >= 0)
    .sort((left, right) => left - right)[0];
  if (assetMarker !== undefined) {
    return normalize(join(root, pathname.slice(assetMarker + 1)));
  }
  const candidate = normalize(join(root, pathname));
  if (candidate === root || candidate.startsWith(`${root}/`)) return candidate;
  return join(root, 'index.html');
};

const startStaticServer = async (root, runtimeProfile) => {
  const server = createServer(async (request, response) => {
    const url = new URL(request.url || '/', 'http://127.0.0.1');
    let filePath = safePath(root, url.pathname);
    try {
      const details = await stat(filePath);
      if (!details.isFile()) filePath = join(root, 'index.html');
    } catch {
      filePath = join(root, 'index.html');
    }
    try {
      const details = await stat(filePath);
      if (runtimeProfile && filePath === join(root, 'index.html')) {
        const html = await readFile(filePath, 'utf8');
        const profileMeta = `<meta name="slskr-runtime-profile" content="${runtimeProfile}">`;
        const body = html.replace('<head>', `<head>${profileMeta}`);
        response.writeHead(200, {
          'cache-control': 'no-store',
          'content-type': 'text/html; charset=utf-8',
          'content-length': Buffer.byteLength(body),
        });
        response.end(body);
        return;
      }
      response.writeHead(200, {
        'cache-control': 'no-store',
        'content-type': contentTypes.get(extname(filePath)) || 'application/octet-stream',
        'content-length': details.size,
      });
      createReadStream(filePath).pipe(response);
    } catch (error) {
      response.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
      response.end(`static UI server error: ${error.message}`);
    }
  });
  await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));
  const address = server.address();
  return { origin: `http://127.0.0.1:${address.port}`, server };
};

const inspectRendered = async (page) => {
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      return await page.evaluate(() => {
        const visible = (element) => {
          const style = window.getComputedStyle(element);
          const box = element.getBoundingClientRect();
          return style.visibility !== 'hidden' && style.display !== 'none' && box.width > 0 && box.height > 0;
        };
        const text = document.body?.innerText?.replace(/\s+/gu, ' ').trim() || '';
        const headings = [...document.querySelectorAll('h1,h2,h3,[role="heading"]')]
          .filter(visible)
          .map((element) => element.textContent?.replace(/\s+/gu, ' ').trim())
          .filter(Boolean)
          .slice(0, 12);
        const buttons = [...document.querySelectorAll('button,[role="button"]')].filter(visible);
        const inputs = [...document.querySelectorAll('input,textarea,select')].filter(visible);
        const links = [...document.querySelectorAll('a[href]')].filter(visible);
        return {
          bodyTextPreview: text.slice(0, 1200),
          buttonCount: buttons.length,
          headingCount: headings.length,
          headings,
          inputCount: inputs.length,
          linkCount: links.length,
          pathname: window.location.pathname,
          title: document.title,
        };
      });
    } catch (error) {
      if (attempt === 2 || !String(error?.message || error).includes('Execution context was destroyed')) {
        throw error;
      }
      await page.waitForTimeout(100);
    }
  }
  throw new Error('render inspection did not complete');
};

const responseShape = (method, url, status, contentType) => ({
  method,
  path: `${url.pathname}${url.search}`,
  status,
  statusClass: `${Math.floor(status / 100)}xx`,
  contentType: contentType.split(';', 1)[0] || '',
});

const captureSurface = async (browser, surface, workflow, server) => {
  // Every workflow/profile pair gets an isolated browser context.  Reusing a
  // context lets cached GET responses and a registered service worker change
  // the observed API inventory based on which workflow happened to run first.
  // That makes an exact path comparison report a cache artifact as a parity
  // defect, especially when two workflows intentionally share a route.
  const context = await browser.newContext({
    serviceWorkers: 'block',
    viewport: { width: 1440, height: 1000 },
  });
  const page = await context.newPage();
  if (surface.runtimeProfile) {
    await page.addInitScript((runtimeProfile) => {
      const meta = document.createElement('meta');
      meta.name = 'slskr-runtime-profile';
      meta.content = runtimeProfile;
      const nativeQuerySelector = Document.prototype.querySelector;
      Document.prototype.querySelector = function querySelector(selector) {
        const result = nativeQuerySelector.call(this, selector);
        return result || selector !== 'meta[name="slskr-runtime-profile"]' ? result : meta;
      };
    }, surface.runtimeProfile);
  }
  const apiResponses = [];
  const pendingApiRequests = new Set();
  let lastApiActivityAt = Date.now();
  const pageErrors = [];
  const consoleErrors = [];
  if (!surface.sameOrigin) {
    await page.addInitScript(() => {
      class AuditWebSocket {
        static CLOSED = 3;

        static CLOSING = 2;

        static CONNECTING = 0;

        static OPEN = 1;

      constructor() {
        this.readyState = AuditWebSocket.OPEN;
        this.url = arguments[0] || '';
        this.protocol = 'json';
        this.handshakeSent = false;
        queueMicrotask(() => this.onopen?.(new Event('open')));
      }

      close() {
        this.readyState = AuditWebSocket.CLOSED;
        queueMicrotask(() => this.onclose?.(new Event('close')));
      }

      send(payload) {
        if (this.handshakeSent || typeof payload !== 'string' || !payload.includes('protocol')) {
          return;
        }
        this.handshakeSent = true;
        queueMicrotask(() => {
          this.onmessage?.({ data: '{}\u001e' });
          const hubPath = this.url.split('?')[0];
          const invocation = (target, argument) => ({
            arguments: [argument],
            target,
            type: 1,
          });
          const emit = (message) => this.onmessage?.({
            data: `${JSON.stringify(message)}\u001e`,
          });
          if (hubPath.includes('/application')) {
            // Both frozen UI builds use the versioned controller API for the
            // state/options snapshot.  The unversioned paths are not part of
            // either target contract and make the slskd profile appear to be
            // permanently disconnected during a side-by-side capture.
            window.__auditApplicationStateReady = Promise.all([
              fetch('/api/v0/application').then((response) => response.json()).catch(() => ({})),
              fetch('/api/v0/options').then((response) => response.json()).catch(() => ({})),
            ]).then(([state, options]) => {
              emit(invocation('state', state));
              emit(invocation('options', options));
            });
          } else if (hubPath.includes('/search')) {
            // The frozen slskd search page stays in its loading state until
            // the hub sends the initial list event.  An empty list is the
            // deterministic disconnected baseline used by this capture. Wait
            // for the application snapshot first so the page receives its
            // server connection state before the search callback renders.
            Promise.resolve(window.__auditApplicationStateReady).then(() => {
              emit(invocation('list', []));
            });
          } else if (hubPath.includes('/metrics')) {
            emit(invocation('Update', {}));
          }
        });
      }
    }

    window.WebSocket = AuditWebSocket;
  });
  }
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().includes('Failed to load resource')) {
      consoleErrors.push(message.text());
    }
  });
  await page.route('**/api/**', async (route) => {
    const request = route.request();
    pendingApiRequests.add(request);
    lastApiActivityAt = Date.now();
    const requestedUrl = new URL(request.url());
    const targetUrl = `${surface.backendUrl}${requestedUrl.pathname}${requestedUrl.search}`;
    const headers = Object.fromEntries(
      Object.entries(request.headers()).filter(
        ([name]) => !['host', 'content-length', 'content-encoding', 'transfer-encoding'].includes(name),
      ),
    );
    try {
      const backendResponse = await fetch(targetUrl, {
        method: request.method(),
        headers,
        body: ['GET', 'HEAD'].includes(request.method()) ? undefined : request.postDataBuffer() || undefined,
      });
      const body = Buffer.from(await backendResponse.arrayBuffer());
      const contentType = backendResponse.headers.get('content-type') || '';
      apiResponses.push(responseShape(request.method(), requestedUrl, backendResponse.status, contentType));
      return route.fulfill({
        body,
        headers: Object.fromEntries(
          [...backendResponse.headers].filter(
            ([name]) => !['content-encoding', 'content-length', 'transfer-encoding'].includes(name),
          ),
        ),
        status: backendResponse.status,
      });
    } catch (error) {
      pageErrors.push(`backend proxy ${requestedUrl.pathname}: ${error.message}`);
      return route.abort('failed');
    } finally {
      pendingApiRequests.delete(request);
      lastApiActivityAt = Date.now();
    }
  });
  if (!surface.sameOrigin) {
    await page.route('**/hub/**', async (route) => {
      const requestedUrl = new URL(route.request().url());
      if (requestedUrl.pathname.endsWith('/negotiate')) {
        return route.fulfill({
          body: JSON.stringify({
            availableTransports: [
              { transport: 'WebSockets', transferFormats: ['Text'] },
            ],
            connectionId: 'slskr-ui-audit',
            connectionToken: 'slskr-ui-audit',
            negotiateVersion: 1,
          }),
          contentType: 'application/json',
          status: 200,
        });
      }
      return route.abort('blockedbyclient');
    });
  }

  const requestedPath = workflow.routes[surface.name];
  const startedAt = new Date().toISOString();
  let navigationError = '';
  try {
    await page.goto(`${server.origin}${requestedPath}`, {
      timeout: 15000,
      waitUntil: 'domcontentloaded',
    });
    await page.waitForTimeout(settleMs);
    const quietDeadline = Date.now() + apiCaptureMaxMs;
    while (
      Date.now() < quietDeadline &&
      (pendingApiRequests.size > 0 || Date.now() - lastApiActivityAt < apiQuietMs)
    ) {
      await page.waitForTimeout(Math.min(50, Math.max(10, apiQuietMs / 4)));
    }
  } catch (error) {
    navigationError = error.message;
  }

  const rendered = await inspectRendered(page);

  const errors = [...pageErrors, ...consoleErrors];
  const actions = [
    {
      type: 'navigate',
      path: requestedPath,
      observedPath: rendered.pathname,
      status: navigationError ? 'fail' : 'ok',
    },
    {
      type: 'inspect-controls',
      buttons: rendered.buttonCount,
      inputs: rendered.inputCount,
      links: rendered.linkCount,
      status: rendered.buttonCount + rendered.inputCount + rendered.linkCount > 0 ? 'ok' : 'fail',
    },
  ];
  const result = {
    actions,
    apiResponses,
    errors,
    evidenceMode: 'live',
    eventFeed: surface.sameOrigin ? 'live' : 'stubbed',
    rendered,
    startedAt,
    surface: surface.name,
    workflow: workflow.id,
  };
  await page.close();
  await context.close();
  return result;
};

const workflowPass = (observations) => {
  return observations.every(
    (observation) =>
      observation.errors.length === 0 &&
      observation.rendered.bodyTextPreview.length > 0 &&
      observation.rendered.buttonCount + observation.rendered.inputCount + observation.rendered.linkCount > 0 &&
      observation.apiResponses.length > 0,
  );
};

const apiPathSet = (observation) =>
  [...new Set(observation.apiResponses.map((response) => `${response.method} ${response.path.split('?')[0]}`))].sort();

const semanticComparison = (workflows) => {
  const comparisons = [];
  const mismatches = [];
  for (const workflow of workflows) {
    const bySurface = Object.fromEntries(
      workflow.observations.map((observation) => [observation.surface, observation]),
    );
    for (const target of ['slskd', 'slskdn']) {
      const targetObservation = bySurface[target];
      const replacementSurface = `replacement-${target}`;
      const replacementObservation = bySurface[replacementSurface];
      if (!targetObservation || !replacementObservation) {
        mismatches.push(`${workflow.id}/${target}: target/profile observation is missing`);
        continue;
      }
      const targetApiPaths = apiPathSet(targetObservation);
      const replacementApiPaths = apiPathSet(replacementObservation);
      const observedPaths = {
        replacement: replacementObservation.rendered.pathname,
        target: targetObservation.rendered.pathname,
      };
      const controlCounts = {
        target: {
          buttons: targetObservation.rendered.buttonCount,
          inputs: targetObservation.rendered.inputCount,
          links: targetObservation.rendered.linkCount,
        },
        replacement: {
          buttons: replacementObservation.rendered.buttonCount,
          inputs: replacementObservation.rendered.inputCount,
          links: replacementObservation.rendered.linkCount,
        },
      };
      const apiPathsEqual = JSON.stringify(targetApiPaths) === JSON.stringify(replacementApiPaths);
      const controlsEqual = JSON.stringify(controlCounts.target) === JSON.stringify(controlCounts.replacement);
      const pathEqual = observedPaths.target === observedPaths.replacement;
      const eventFeedLive = replacementObservation.eventFeed === 'live';
      const comparison = {
        apiPathsEqual,
        controlCounts,
        controlsEqual,
        eventFeedLive,
        observedPaths,
        pathEqual,
        replacementApiPaths,
        replacementSurface,
        target,
        targetApiPaths,
        workflow: workflow.id,
      };
      comparisons.push(comparison);
      if (!apiPathsEqual) mismatches.push(`${workflow.id}/${target}: API path inventory differs`);
      if (!controlsEqual) mismatches.push(`${workflow.id}/${target}: visible control inventory differs`);
      if (!pathEqual) mismatches.push(`${workflow.id}/${target}: observed route differs`);
      if (!eventFeedLive) mismatches.push(`${workflow.id}/${target}: replacement event feed is not live`);
    }
  }
  const replacementProfiles = Object.values(surfaces)
    .filter((surface) => surface.profile)
    .map((surface) => surface.profile)
    .sort();
  if (JSON.stringify(replacementProfiles) !== JSON.stringify(['slskd', 'slskdn'])) {
    mismatches.push('replacement compatibility profile matrix is incomplete');
  }
  return {
    comparisonBasis: 'exact observed route, API path inventory, visible control inventory, live event feed, and profile matrix',
    comparisons,
    replacementEventFeed: workflows.every((workflow) =>
      ['replacement-slskd', 'replacement-slskdn'].every((surfaceName) =>
        workflow.observations.some(
          (observation) => observation.surface === surfaceName && observation.eventFeed === 'live',
        ),
      ),
    ) ? 'live' : 'stubbed',
    replacementProfiles,
    status: mismatches.length === 0 ? 'pass' : 'fail',
    mismatches,
  };
};

const main = async () => {
  const missing = requiredEnvironment.filter((name) => !process.env[name]);
  if (missing.length > 0) fail(`missing required environment: ${missing.join(', ')}`);
  for (const [name, surface] of Object.entries(surfaces)) {
    surface.name = name;
    surface.backendUrl = assertBackend(surface.backendUrl, `${name} backend URL`);
    await assertDirectory(surface.uiRoot, `${name} UI root`);
  }
  for (const target of ['slskd', 'slskdn']) {
    const targetRoot = resolve(surfaces[target].uiRoot);
    const replacementRoot = resolve(surfaces[`replacement-${target}`].uiRoot);
    if (targetRoot === replacementRoot) {
      fail(
        `replacement-${target} must use an independently built replacement UI root; ` +
          `it currently points at the frozen target UI root ${targetRoot}`,
      );
    }
  }

  const servers = {};
  for (const [name, surface] of Object.entries(surfaces)) {
    servers[name] = surface.sameOrigin
      ? { origin: surface.backendUrl, server: null }
      : await startStaticServer(resolve(surface.uiRoot), surface.profile);
  }
  let browser;
  try {
    browser = await chromium.launch({
      executablePath: browserExecutablePath,
      headless: true,
    });
  } catch (error) {
    for (const { server } of Object.values(servers)) server?.close();
    throw error;
  }
  const evidence = {
    comparisonMode: 'frozen-target-side-by-side',
    evidenceMode: 'live',
    generatedAt: new Date().toISOString(),
    replacement: 'slskR',
    targets: ['slskd', 'slskdn'],
    workflows: [],
  };
  let failedWorkflows = 0;
  try {
    for (const workflow of workflows) {
      const observations = [];
      for (const name of ['slskd', 'slskdn', 'replacement-slskd', 'replacement-slskdn']) {
        observations.push(await captureSurface(browser, surfaces[name], workflow, servers[name]));
      }
      const pass = workflowPass(observations);
      if (!pass) failedWorkflows += 1;
      evidence.workflows.push({
        actions: observations.flatMap((observation) =>
          observation.actions.map((action) => ({ ...action, surface: observation.surface })),
        ),
        id: workflow.id,
        observations,
        responses: observations.flatMap((observation) =>
          observation.apiResponses.map((response) => ({ ...response, surface: observation.surface })),
        ),
        status: pass ? 'pass' : 'fail',
        targets: ['slskd', 'slskdn'],
      });
    }
  } finally {
    await browser.close();
    for (const { server } of Object.values(servers)) server?.close();
  }
  evidence.semanticComparison = semanticComparison(evidence.workflows);

  await mkdir(resolve(outputPath, '..'), { recursive: true });
  const artifacts = [];
  for (const [name, surface] of Object.entries(surfaces)) {
    artifacts.push({ name, uiRoot: resolve(surface.uiRoot), backendUrl: surface.backendUrl });
  }
  evidence.artifacts = artifacts.map((artifact) => join(artifact.uiRoot, 'index.html'));
  evidence.surfaceInputs = artifacts;
  await import('node:fs/promises').then(({ writeFile }) =>
    writeFile(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, 'utf8'),
  );
  console.log(
    `frozen target UI comparison: ${workflows.length - failedWorkflows}/${workflows.length} workflows pass; output=${outputPath}`,
  );
  if (requirePass && failedWorkflows > 0) process.exitCode = 1;
};

main().catch((error) => {
  console.error(`frozen target UI comparison failed: ${error.message}`);
  process.exitCode = 2;
});
