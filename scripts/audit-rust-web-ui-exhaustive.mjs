#!/usr/bin/env node

import { createServer } from 'node:http';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require('../web/node_modules/playwright');

const repoRoot = resolve(new URL('..', import.meta.url).pathname);
const distDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_DIST || 'target/slskr-web');
const outputDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_EXHAUSTIVE_DIR || 'target/ux-audit/current-exhaustive');
const backendUrl = (process.env.SLSKR_RUST_WEB_AUDIT_BACKEND_URL || '').replace(/\/$/, '');
const authHeader = process.env.SLSKR_RUST_WEB_AUDIT_AUTH_HEADER || '';
const settleMs = Math.max(200, Number.parseInt(process.env.SLSKR_RUST_WEB_EXHAUSTIVE_SETTLE_MS || '500', 10) || 500);
const routeFilter = (process.env.SLSKR_RUST_WEB_EXHAUSTIVE_ROUTES || '').split(',').map((route) => route.trim()).filter(Boolean);
const searchDetailId = process.env.SLSKR_RUST_WEB_EXHAUSTIVE_SEARCH_ID || '1';

const routes = [
  ['/', 'Search'],
  ['/searches', 'Search'],
  [`/searches/${searchDetailId}`, 'Search Detail'],
  ['/discovery-graph', 'Discovery Graph'],
  ['/playlist-intake', 'Playlist Intake'],
  ['/wishlist', 'Wishlist'],
  ['/downloads', 'Downloads'],
  ['/uploads', 'Uploads'],
  ['/messages', 'Messages'],
  ['/chat', 'Chat'],
  ['/rooms', 'Rooms'],
  ['/users', 'Users'],
  ['/contacts', 'Contacts'],
  ['/solid', 'Solid'],
  ['/collections', 'Collections'],
  ['/sharegroups', 'Share Groups'],
  ['/shared', 'Shared with Me'],
  ['/browse', 'Browse'],
  ['/system', 'System'],
  ['/system/network', 'System Tab'],
  ['/pods', 'Pods'],
  ['/pods/demo', 'Pod Redirect'],
  ['/pods/demo/channels/general', 'Pod Channel Redirect'],
];
const selectedRoutes = routeFilter.length ? routes.filter(([path]) => routeFilter.includes(path)) : routes;
const searchDetailPath = `/searches/${searchDetailId}`;

const contentTypes = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.html', 'text/html; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.wasm', 'application/wasm'],
]);

const mockBody = (path, method) => {
  if (path.includes('/health')) return { service: 'slskr', status: 'ok' };
  if (path.includes('/version')) return { name: 'slskR', version: 'audit' };
  if (path.includes('/application')) return { pendingRestart: false, relay: { enabled: false } };
  if (path.includes('/server')) return { isConnected: true, isLoggedIn: true, username: 'audit-user' };
  if (path.includes('/nowplaying')) return { filename: 'audit.flac', peer: 'peer-audit', state: 'playing', contentId: 'sha256:audit' };
  if (path.includes('/transfers/speeds')) return { download: 98304, upload: 24576 };
  if (path.includes('/transfers/downloads')) return [{ id: 1, filename: 'audit-track.flac', username: 'peer-audit', progress: 0.58, state: 'Active', size: 42152704 }];
  if (path.includes('/transfers/uploads')) return [{ id: 1, filename: 'shared-track.flac', username: 'listener-audit', progress: 0.32, state: 'Queued', size: 35127296 }];
  if (path.includes('/searches/1/responses') || path.includes('/searches/:id/responses')) return [{ filename: 'Artist - Audit.flac', username: 'peer-audit', size: 41800000, bitrate: 1011, queue: 0, speed: 512000 }];
  if (path.includes('/searches/records') || path.endsWith('/searches')) return [{ id: 1, searchText: 'Artist Audit', state: 'Complete', responseCount: 1 }];
  if (path.includes('/wishlist')) return [{ id: 'want-1', searchText: 'rare audit pressing', enabled: true, autoDownload: false, resultCount: 3 }];
  if (path.includes('/conversations')) return [{ username: 'chat-audit', unread: 1, lastMessage: 'Still available?', lastMessageAt: '2026-05-06T12:00:00Z' }];
  if (path.includes('/rooms')) return [{ name: 'audit-room', userCount: 7 }];
  if (path.includes('/users/') && path.includes('/browse')) return { username: 'peer-audit', folders: [{ name: 'Music', files: [{ filename: 'audit.flac', size: 1234 }] }] };
  if (path.includes('/users')) return [{ username: 'peer-audit', status: 'Online', privileged: true, files: 128 }];
  if (path.includes('/contacts')) return [{ username: 'friend-audit', status: 'Online', group: 'Friends', note: 'trusted' }];
  if (path.includes('/solid/status')) return { enabled: true, webId: 'https://audit.example/profile/card#me', storage: 'ready' };
  if (path.includes('/collections')) return [{ id: 'collection-1', title: 'Audit Collection', itemCount: 2, owner: 'audit-user' }];
  if (path.includes('/sharegroups')) return [{ id: 'group-1', name: 'Audit Group', members: 2, permissions: 'read,stream' }];
  if (path.includes('/shared')) return [{ id: 'grant-1', owner: 'friend-audit', title: 'Shared Audit', permissions: 'stream', expiresAt: null }];
  if (path.includes('/source-providers')) return [{ id: 'provider-1', name: 'MusicBrainz', enabled: true }];
  if (path.includes('/jobs')) return [{ id: 'job-1', type: 'scan', state: 'Complete' }];
  if (path.includes('/shares')) return { roots: 1, files: 128, scanState: 'Idle' };
  if (path.includes('/database/stats')) return { tracks: 128, peers: 7, status: 'ready' };
  if (path.includes('/logs')) return [{ level: 'info', message: 'audit log' }];
  if (path.includes('/telemetry') || path.includes('/metrics')) return { uptimeSeconds: 60 };
  if (method !== 'GET') return { ok: true, id: 'audit-mutation' };
  return [];
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

const textSnapshot = async (page) => page.locator('body').innerText().catch(() => '');

const controlDescription = async (locator) => locator.evaluate((element) => ({
  aria: element.getAttribute('aria-label') || '',
  data: [...element.attributes].filter(({ name }) => name.startsWith('data-')).map(({ name, value }) => `${name}=${value}`).join(';'),
  disabled: element.disabled === true || element.hasAttribute('disabled'),
  href: element.getAttribute('href') || '',
  label: (element.innerText || element.value || element.getAttribute('title') || '').trim().replace(/\s+/g, ' ').slice(0, 160),
  tag: element.tagName.toLowerCase(),
  type: element.getAttribute('type') || '',
}));

const locatorForDescriptor = (page, descriptor, index) => {
  const dataAttribute = descriptor.data.split(';').find((entry) => entry.startsWith('data-'));
  if (dataAttribute) {
    const separator = dataAttribute.indexOf('=');
    const name = separator === -1 ? dataAttribute : dataAttribute.slice(0, separator);
    const value = separator === -1 ? '' : dataAttribute.slice(separator + 1).replaceAll('"', '\\"');
    return page.locator(value ? `button:visible[${name}="${value}"]` : `button:visible[${name}]`).first();
  }
  if (descriptor.label) return page.getByRole('button', { name: descriptor.label, exact: true }).first();
  return page.locator('button:visible').nth(index);
};

const visibleSelector = 'button:visible, a:visible, input:visible, select:visible, textarea:visible';
const liveStateSelector = '#slskr-page-data[data-slskr-live-state="ready"], #slskr-page-data[data-slskr-live-state="error"]';

const resetLiveSearchDetail = async () => {
  if (!backendUrl) return;
  const headers = { 'content-type': 'application/json' };
  if (authHeader) headers.authorization = authHeader;
  await fetch(`${backendUrl}/api/v0/searches/${encodeURIComponent(searchDetailId)}`, {
    method: 'DELETE',
    headers,
  }).catch(() => {});
  await fetch(`${backendUrl}/api/v0/searches`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ id: searchDetailId, searchText: 'browser evidence detail' }),
  });
  const response = await fetch(`${backendUrl}/api/v0/searches/${encodeURIComponent(searchDetailId)}`, {
    headers,
  });
  if (!response.ok) throw new Error(`search detail fixture returned HTTP ${response.status}`);
};

const main = async () => {
  if (!existsSync(join(distDir, 'index.html'))) throw new Error(`missing web distribution: ${distDir}`);
  const server = await startStaticServer();
  await mkdir(outputDir, { recursive: true });
  const { port } = server.address();
  const browser = await chromium.launch({ headless: process.env.HEADLESS !== 'false' });
  const evidence = {
    backend: backendUrl ? 'live' : 'mock',
    controls: [],
    errors: [],
    generatedAt: new Date().toISOString(),
    links: [],
    routes: [],
  };

  try {
    const makePage = async () => {
      const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
      const requests = [];
      const responses = [];
      const pageErrors = [];
      page.on('request', (request) => {
        if (request.url().includes('/api/')) requests.push(`${request.method()} ${new URL(request.url()).pathname}`);
      });
      page.on('response', (response) => {
        if (response.url().includes('/api/')) {
          responses.push({
            method: response.request().method(),
            path: new URL(response.url()).pathname,
            status: response.status(),
          });
        }
      });
      page.on('pageerror', (error) => pageErrors.push(error.message));
      page.on('console', (message) => {
        if (message.type() === 'error' && !message.text().includes('Failed to load resource')) pageErrors.push(message.text());
      });
      await page.route('**/api/**', async (route) => {
        const request = route.request();
        const requestedUrl = new URL(request.url());
        if (!backendUrl) {
          return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(mockBody(requestedUrl.pathname, request.method())) });
        }
        const targetUrl = `${backendUrl}${requestedUrl.pathname}${requestedUrl.search}`;
        const headers = Object.fromEntries(Object.entries(request.headers()).filter(([name]) => !['host', 'content-length', 'content-encoding', 'transfer-encoding'].includes(name)));
        if (authHeader) headers.authorization = authHeader;
        try {
          const upstream = await fetch(targetUrl, {
            method: request.method(),
            headers,
            body: ['GET', 'HEAD'].includes(request.method()) ? undefined : request.postDataBuffer() || undefined,
          });
          const body = Buffer.from(await upstream.arrayBuffer());
          return route.fulfill({ status: upstream.status, headers: Object.fromEntries([...upstream.headers].filter(([name]) => !['content-encoding', 'content-length', 'transfer-encoding'].includes(name))), body });
        } catch (error) {
          evidence.errors.push(`proxy ${request.method()} ${requestedUrl.pathname}: ${error.message}`);
          return route.abort('failed');
        }
      });
      return { page, pageErrors, requests, responses };
    };

    for (const [path, expectedHeading] of selectedRoutes) {
      console.error(`auditing ${path}`);
      const { page, pageErrors, requests, responses } = await makePage();
      try {
        if (path === searchDetailPath) await resetLiveSearchDetail();
        await page.goto(`http://127.0.0.1:${port}${path}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.locator(liveStateSelector).waitFor({ state: 'attached', timeout: 5000 });
        await page.waitForTimeout(50);
        const heading = await page.locator('.slskr-page-header h2').innerText().catch(() => '');
        const readyState = await page.locator('#slskr-page-data').getAttribute('data-slskr-live-state').catch(() => 'missing');
        const rawApiText = await page.locator('body').innerText();
        const controls = await page.locator(visibleSelector).evaluateAll((nodes) => nodes.map((element, index) => ({
          index,
          aria: element.getAttribute('aria-label') || '',
          data: [...element.attributes].filter(({ name }) => name.startsWith('data-')).map(({ name, value }) => `${name}=${value}`).join(';'),
          disabled: element.disabled === true || element.hasAttribute('disabled'),
          href: element.getAttribute('href') || '',
          label: (element.innerText || element.value || element.getAttribute('title') || '').trim().replace(/\s+/g, ' ').slice(0, 160),
          mounted: element.getAttribute('data-slskr-mounted') || '',
          selected: element.getAttribute('aria-selected') || '',
          tag: element.tagName.toLowerCase(),
          type: element.getAttribute('type') || '',
        })));
        const links = controls.filter((control) => control.tag === 'a');
        evidence.routes.push({
          path,
          expectedHeading,
          heading,
          readyState,
          requests: [...requests],
          responseStatuses: [...responses],
          controls: controls.length,
          links: links.length,
          pageErrors: [...pageErrors],
        });
        if (heading !== expectedHeading) evidence.errors.push(`${path}: expected heading ${expectedHeading}, got ${heading || '<empty>'}`);
        if (readyState !== 'ready' && !(backendUrl && readyState === 'error')) {
          evidence.errors.push(`${path}: live workspace state is ${readyState}`);
        }
        if (rawApiText.includes('GET /api/v0')) evidence.errors.push(`${path}: raw API contract text leaked into visible page`);
        if (pageErrors.length) evidence.errors.push(`${path}: browser errors: ${pageErrors.join(' | ')}`);

        for (const link of links) {
          if (!link.href) {
            evidence.errors.push(`${path}: link has no href (${link.label || link.aria || '<unlabelled>'})`);
            continue;
          }
          if (link.href.startsWith('/') || link.href.startsWith(`http://127.0.0.1:${port}`)) {
            evidence.links.push({ path, label: link.label, href: link.href, kind: 'internal' });
          } else if (/^https?:\/\//.test(link.href)) {
            evidence.links.push({ path, label: link.label, href: link.href, kind: 'external' });
          } else if (!link.href.startsWith('#')) {
            evidence.errors.push(`${path}: unsupported link target ${link.href}`);
          }
        }

        const buttons = controls.filter((control) => control.tag === 'button');
        for (let index = 0; index < buttons.length; index += 1) {
          const descriptor = buttons[index];
          if (descriptor.disabled) {
            evidence.controls.push({ path, index, ...descriptor, result: 'disabled' });
            continue;
          }
          if (path === searchDetailPath) await resetLiveSearchDetail();
          await page.goto(`http://127.0.0.1:${port}${path}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
          await page.locator(liveStateSelector).waitFor({ state: 'attached', timeout: 5000 });
          // The page reload above is setup for this control.  Reset the
          // event ledgers after it so request/response evidence belongs to
          // the click being exercised rather than to the route bootstrap.
          requests.length = 0;
          responses.length = 0;
          const beforeUrl = page.url();
          const beforeText = await textSnapshot(page);
          const beforeHtmlLength = await page.locator('body').evaluate((body) => body.innerHTML.length);
          const beforeDom = await page.locator('body').evaluate((body) => body.innerHTML);
          const beforeActive = await page.evaluate(() => document.activeElement?.outerHTML || '');
          const beforeStatus = await page.locator('#slskr-action-status, #slskr-toast-region, [aria-live="polite"]').allTextContents();
          const beforeModal = await page.locator('.slskr-modal-backdrop:visible, dialog:visible').count();
          const requestStart = requests.length;
          const responseStart = responses.length;
          let fileChooser = false;
          const dragControl = descriptor.data.includes('data-slskr-transfer-column-resize=');
          page.once('filechooser', () => { fileChooser = true; });
          let clickError = '';
          try {
            const button = locatorForDescriptor(page, descriptor, index);
            if (!(await button.count())) {
              evidence.controls.push({ path, index, ...descriptor, result: 'not-present-after-previous-action' });
              continue;
            }
            if (dragControl) {
              await button.scrollIntoViewIfNeeded();
              const box = await button.boundingBox();
              if (!box) throw new Error('resize handle has no layout box');
              const x = box.x + Math.max(1, box.width / 2);
              const y = box.y + Math.max(1, box.height / 2);
              await page.mouse.move(x, y);
              await page.mouse.down();
              await page.mouse.move(x + 48, y);
              await page.mouse.up();
            } else {
              await button.evaluate((element) => element.click());
            }
          } catch (error) {
            clickError = error.message;
          }
          await page.waitForTimeout(Math.min(350, settleMs));
          const afterUrl = page.url();
          const afterText = await textSnapshot(page);
          const afterHtmlLength = await page.locator('body').evaluate((body) => body.innerHTML.length).catch(() => 0);
          const afterDom = await page.locator('body').evaluate((body) => body.innerHTML).catch(() => '');
          const afterActive = await page.evaluate(() => document.activeElement?.outerHTML || '').catch(() => '');
          const tabData = descriptor.data.split(';').find((entry) => entry.startsWith('data-slskr-native-tab='));
          const [tabName, tabValue] = tabData ? tabData.split('=') : ['', ''];
          const selectedAfter = tabData
            ? await page.locator(`button:visible[${tabName}="${tabValue}"]`).first().getAttribute('aria-selected').catch(() => '')
            : '';
          const afterStatus = await page.locator('#slskr-action-status, #slskr-toast-region, [aria-live="polite"]').allTextContents();
          const afterModal = await page.locator('.slskr-modal-backdrop:visible, dialog:visible').count();
          const requestDelta = requests.slice(requestStart);
          const responseDelta = responses.slice(responseStart);
          const httpFailures = responseDelta.filter(({ status }) => status >= 400);
          const changed = beforeUrl !== afterUrl || beforeText !== afterText || beforeHtmlLength !== afterHtmlLength || beforeDom !== afterDom || beforeActive !== afterActive || JSON.stringify(beforeStatus) !== JSON.stringify(afterStatus) || beforeModal !== afterModal || fileChooser;
          const allowedNoOp = (descriptor.data.includes('data-slskr-native-filter-clear') && !beforeText.includes('Filter:'))
            || (descriptor.data.includes('data-slskr-native-tab=') && descriptor.selected === 'true');
          const result = clickError ? 'click-error' : (changed || requestDelta.length > 0 ? (dragControl ? 'drag-transitioned' : 'transitioned') : (allowedNoOp ? 'allowed-no-op' : 'no-observable-effect'));
          evidence.controls.push({
            path,
            index,
            ...descriptor,
            result,
            changed,
            requests: requestDelta.length,
            requestPaths: requestDelta,
            responseStatuses: responseDelta,
            httpFailures,
            beforeUrl,
            afterUrl,
            fileChooser,
            clickError,
            selectedAfter,
          });
          if (result === 'click-error') evidence.errors.push(`${path} button ${index} ${descriptor.label || descriptor.aria || '<unlabelled>'}: ${clickError}`);
          if (result === 'no-observable-effect') evidence.errors.push(`${path} button ${index} ${descriptor.label || descriptor.aria || '<unlabelled>'}: click had no request, navigation, modal, status, toast, or DOM transition`);
          if (process.env.SLSKR_RUST_WEB_EXHAUSTIVE_FAIL_ON_HTTP_ERROR === 'true' && httpFailures.length) {
            evidence.errors.push(`${path} button ${index} ${descriptor.label || descriptor.aria || '<unlabelled>'}: HTTP ${httpFailures.map(({ status, method, path: responsePath }) => `${status} ${method} ${responsePath}`).join(', ')}`);
          }
        }
      } finally {
        await page.close();
      }
    }

    await writeFile(join(outputDir, 'exhaustive-audit.json'), `${JSON.stringify(evidence, null, 2)}\n`);
  } finally {
    await browser.close();
    server.close();
  }

  const summary = {
    routes: evidence.routes.length,
    controls: evidence.controls.length,
    transitioned: evidence.controls.filter((control) => control.result === 'transitioned').length,
    disabled: evidence.controls.filter((control) => control.result === 'disabled').length,
    failures: evidence.errors.length,
    output: join(outputDir, 'exhaustive-audit.json'),
  };
  console.log(JSON.stringify(summary, null, 2));
  if (evidence.errors.length) {
    console.error(evidence.errors.join('\n'));
    process.exit(1);
  }
};

await main();
