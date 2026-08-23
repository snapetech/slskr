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
const backendUrl = (process.env.SLSKR_RUST_WEB_AUDIT_BACKEND_URL || 'http://127.0.0.1:58071').replace(/\/$/, '');
const outputDir = resolve(repoRoot, process.env.SLSKR_RUST_WEB_SEMANTIC_DIR || 'target/ux-audit/current-semantic');
const contentTypes = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.html', 'text/html; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.wasm', 'application/wasm'],
]);

const assert = (condition, message) => {
  if (!condition) throw new Error(message);
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

const apiRequest = async (method, path, body, headers = {}) => {
  const response = await fetch(`${backendUrl}/api/v0${path}`, {
    method,
    headers: { ...(body === undefined ? {} : { 'content-type': 'application/json' }), ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    json = null;
  }
  return { json, status: response.status, text };
};

const main = async () => {
  assert(existsSync(join(distDir, 'index.html')), `missing web distribution: ${distDir}`);
  const server = await startStaticServer();
  await mkdir(outputDir, { recursive: true });
  const { port } = server.address();
  const browser = await chromium.launch({ headless: true });
  const evidence = {
    backend: backendUrl,
    generatedAt: new Date().toISOString(),
    checks: [],
    errors: [],
  };
  let collectionId;
  let grantId;
  let stage = 'startup';

  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    page.on('popup', (popup) => popup.close().catch(() => {}));
    await page.route('**/api/**', async (route) => {
      const request = route.request();
      const requestedUrl = new URL(request.url());
      const targetUrl = `${backendUrl}${requestedUrl.pathname}${requestedUrl.search}`;
      const headers = Object.fromEntries(
        Object.entries(request.headers()).filter(([name]) => !['host', 'content-length', 'content-encoding', 'transfer-encoding'].includes(name)),
      );
      try {
        const upstream = await fetch(targetUrl, {
          method: request.method(),
          headers,
          body: ['GET', 'HEAD'].includes(request.method()) ? undefined : request.postDataBuffer() || undefined,
        });
        const body = Buffer.from(await upstream.arrayBuffer());
        return route.fulfill({
          status: upstream.status,
          headers: Object.fromEntries([...upstream.headers].filter(([name]) => !['content-encoding', 'content-length', 'transfer-encoding'].includes(name))),
          body,
        });
      } catch (error) {
        evidence.errors.push(`proxy ${request.method()} ${requestedUrl.pathname}: ${error.message}`);
        return route.abort('failed');
      }
    });

    const goto = async (path) => {
      await page.goto(`http://127.0.0.1:${port}${path}`, { waitUntil: 'domcontentloaded', timeout: 30000 });
      await page.locator('#slskr-page-data').waitFor({ state: 'attached', timeout: 10000 });
      await page.waitForFunction(() => {
        const element = document.querySelector('#slskr-page-data');
        return element && element.getAttribute('data-slskr-live-state') !== 'pending';
      }, null, { timeout: 30000 });
    };

    const visibleButton = (label) => page.locator('button:visible').filter({ hasText: label }).filter({ has: page.locator('text=' + label) }).first();
    const exactButton = (label) => page.getByRole('button', { name: label, exact: true }).first();
    const tab = (label) => page.getByRole('tab', { name: label, exact: true }).first();

    const clickApi = async (label, method, path, locator = exactButton(label)) => {
      await locator.waitFor({ state: 'visible', timeout: 10000 });
      const responsePromise = page.waitForResponse((response) => {
        const url = new URL(response.url());
        return response.request().method() === method && url.pathname === `/api/v0${path}`;
      }, { timeout: 15000 });
      await locator.click();
      const response = await responsePromise;
      assert(response.status() >= 200 && response.status() < 300, `${label}: ${method} ${path} returned ${response.status()}`);
      evidence.checks.push({ label, method, path, status: response.status() });
      return response;
    };

    stage = 'contacts';
    await goto('/contacts');
    await clickApi('Create Invite', 'POST', '/profile/invite');
    await clickApi('Refresh Nearby', 'GET', '/contacts/nearby');

    stage = 'system-options';
    await goto('/system');
    const yamlValidationPromise = page.waitForResponse((response) => response.request().method() === 'POST' && new URL(response.url()).pathname === '/api/v0/options/yaml/validate', { timeout: 15000 });
    await page.locator('[data-slskr-options-yaml-validate]:visible').click();
    const yamlValidation = await yamlValidationPromise;
    assert(yamlValidation.status() >= 200 && yamlValidation.status() < 300, `Validate YAML returned ${yamlValidation.status()}`);
    evidence.checks.push({ label: 'Validate YAML', method: 'POST', path: '/options/yaml/validate', status: yamlValidation.status() });
    const yamlSavePromise = page.waitForResponse((response) => response.request().method() === 'PUT' && new URL(response.url()).pathname === '/api/v0/options/yaml', { timeout: 15000 });
    await page.locator('[data-slskr-options-yaml-save]:visible').click();
    const yamlSave = await yamlSavePromise;
    assert(yamlSave.status() >= 200 && yamlSave.status() < 300, `Save YAML returned ${yamlSave.status()}`);
    evidence.checks.push({ label: 'Save YAML', method: 'PUT', path: '/options/yaml', status: yamlSave.status() });

    stage = 'library-health';
    await tab('Library Health').click();
    const libraryPath = page.locator('input[aria-label="Library Path"]:visible').first();
    await libraryPath.fill('/tmp/slskr-completeness-audit/shared');
    await clickApi('Scan Library Health', 'POST', '/library/health/scans');
    await clickApi('Fix Library Issues', 'POST', '/library/health/issues/fix');

    stage = 'experience';
    await tab('Experience').click();
    await page.locator('[data-slskr-pref-action="save"]:visible').click();
    await page.locator('[data-slskr-pref-action="copy"]:visible').click();
    await page.waitForFunction(() => document.querySelector('#slskr-experience-report')?.textContent?.includes('slskr experience preferences') || false, null, { timeout: 5000 });
    evidence.checks.push({ label: 'Experience Copy Report', kind: 'local', result: 'report generated' });

    stage = 'automations';
    await tab('Automations').click();
    await page.locator('[data-slskr-recipe-copy="library-health-scan"]:visible').click();
    await page.waitForFunction(() => document.querySelector('#slskr-automation-status')?.textContent?.includes('plan copied') || false, null, { timeout: 5000 });
    evidence.checks.push({ label: 'Automation Copy Plan', kind: 'local', result: 'plan generated and status surfaced' });

    stage = 'wishlist';
    await goto('/wishlist');
    await page.locator('.wishlist-native textarea[aria-label="Search Text"]:visible').first().fill('semantic-ui-wanted-one\nsemantic-ui-wanted-two');
    const wishlistResponses = [];
    const responseListener = (response) => {
      const url = new URL(response.url());
      if (response.request().method() === 'POST' && url.pathname === '/api/v0/wishlist') wishlistResponses.push({ status: response.status(), body: response.request().postData() || '' });
    };
    page.on('response', responseListener);
    await page.locator('.wishlist-native button:visible').filter({ hasText: 'Import List' }).first().click();
    await page.waitForTimeout(1500);
    page.off('response', responseListener);
    if (wishlistResponses.length !== 2) {
      evidence.debug = {
        wishlistInputs: await page.locator('input[aria-label="Search Text"]').evaluateAll((nodes) => nodes.map((node) => ({ value: node.value, classes: node.className, parent: node.parentElement?.className || '' }))),
        wishlistButtons: await page.locator('button:visible').filter({ hasText: 'Import List' }).evaluateAll((nodes) => nodes.map((node) => ({ text: node.innerText, classes: node.className, parent: node.parentElement?.className || '' }))),
        wishlistBody: await page.locator('body').innerText(),
      };
    }
    assert(wishlistResponses.length === 2 && wishlistResponses.every(({ status }) => status >= 200 && status < 300), `Import List responses: ${JSON.stringify(wishlistResponses)}`);
    evidence.checks.push({ label: 'Import List', method: 'POST', path: '/wishlist', requests: wishlistResponses.length, statuses: wishlistResponses.map(({ status }) => status), payloads: wishlistResponses.map(({ body }) => body) });

    stage = 'shared-fixture';
    const collection = await apiRequest('POST', '/collections', { title: `semantic-ui-${Date.now()}`, description: 'semantic browser proof' });
    assert(collection.status === 201 && collection.json?.id, `shared collection create returned ${collection.status}`);
    collectionId = collection.json.id;
    const item = await apiRequest('POST', `/collections/${encodeURIComponent(collectionId)}/items`, { contentId: `semantic-ui-content-${Date.now()}`, title: 'semantic-ui-track' });
    assert(item.status === 201 && item.json?.contentId, `shared item create returned ${item.status}`);
    const grant = await apiRequest('POST', '/share-grants', { collectionId, username: 'semantic-ui-peer' });
    assert(grant.status === 201 && grant.json?.id, `shared grant create returned ${grant.status}`);
    grantId = grant.json.id;

    stage = 'shared-ui';
    await goto('/shared');
    const sharedRow = page.locator(`[data-slskr-native-select][data-slskr-native-grant-id="${grantId}"]:visible`).first();
    await sharedRow.waitFor({ state: 'visible', timeout: 10000 });
    await sharedRow.click();
    await clickApi('Copy token', 'POST', `/share-grants/${grantId}/token`);
    await clickApi('Open', 'GET', `/share-grants/${grantId}/manifest`);
    // Opening the manifest refreshes the live route data. Re-select the grant
    // after that refresh so Stream has a concrete row identity to resolve.
    const streamRow = page.locator(`[data-slskr-native-select][data-slskr-native-grant-id="${grantId}"]:visible`).first();
    await streamRow.waitFor({ state: 'visible', timeout: 10000 });
    await streamRow.click();
    const streamButton = page.locator('.shared-native button:visible').filter({ hasText: 'Stream' }).first();
    await clickApi('Stream', 'POST', `/streams/${encodeURIComponent(item.json.contentId)}/share-ticket`, streamButton);
    evidence.checks.push({ label: 'Shared stream ticket', method: 'POST', path: `/streams/${encodeURIComponent(item.json.contentId)}/share-ticket`, result: 'ticket exchange completed' });

    const unwanted = await apiRequest('GET', '/wishlist');
    for (const entry of unwanted.json || []) {
      if (String(entry.searchText || '').startsWith('semantic-ui-wanted-')) {
        await apiRequest('DELETE', `/wishlist/${encodeURIComponent(entry.id)}`);
      }
    }
  } catch (error) {
    evidence.errors.push(`${stage}: ${error.message}`);
  } finally {
    if (grantId) await apiRequest('DELETE', `/share-grants/${grantId}`);
    if (collectionId) await apiRequest('DELETE', `/collections/${collectionId}`);
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
  }

  await writeFile(join(outputDir, 'semantic-audit.json'), `${JSON.stringify(evidence, null, 2)}\n`);
  if (evidence.errors.length) {
    console.error(JSON.stringify(evidence, null, 2));
    process.exitCode = 1;
  } else {
    console.log(JSON.stringify(evidence, null, 2));
  }
};

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
