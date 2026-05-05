import { chromium } from 'playwright';

const base = process.env.UI_BASE || 'http://127.0.0.1:3001';
const routes = [
  '/', '/searches', '/discovery-graph', '/playlist-intake', '/wishlist',
  '/downloads', '/uploads', '/messages', '/users', '/contacts', '/solid',
  '/collections', '/sharegroups', '/shared', '/browse', '/chat', '/pods',
  '/rooms', '/system', '/system/info', '/system/options', '/system/shares',
  '/system/logs', '/system/network', '/system/integrations', '/system/security',
  '/system/admin', '/system/media', '/system/mesh', '/system/jobs',
];
const ignoreConsole = [
  /Support for defaultProps will be removed/,
  /findDOMNode is deprecated/,
  /React Router Future Flag Warning/,
  /Download the React DevTools/,
];
const ignoreFailed = [
  /fonts\.gstatic\.com/,
  /fonts\.googleapis\.com/,
];

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const summary = [];

for (const route of routes) {
  const page = await context.newPage();
  const events = [];
  page.on('console', msg => {
    const text = msg.text();
    if (['error', 'warning'].includes(msg.type()) && !ignoreConsole.some(r => r.test(text))) {
      events.push({ type: `console:${msg.type()}`, text: text.slice(0, 500) });
    }
  });
  page.on('pageerror', err => events.push({ type: 'pageerror', text: err.message }));
  page.on('requestfailed', req => {
    const url = req.url();
    if (!ignoreFailed.some(r => r.test(url))) {
      events.push({ type: 'requestfailed', text: `${req.method()} ${url} ${req.failure()?.errorText}` });
    }
  });
  page.on('response', res => {
    const url = res.url();
    if (res.status() >= 400 && url.startsWith(base)) {
      events.push({ type: 'response', text: `${res.status()} ${url}` });
    }
  });
  let body = '';
  let title = '';
  try {
    const response = await page.goto(`${base}${route}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
    if (!response || response.status() >= 400) {
      events.push({ type: 'navigation', text: `${response?.status()} ${base}${route}` });
    }
    await page.waitForTimeout(2500);
    title = await page.title();
    body = (await page.locator('body').innerText({ timeout: 5000 }).catch(() => '')).trim();
    const rootHtml = await page.locator('#root').innerHTML({ timeout: 5000 }).catch(() => '');
    if (!rootHtml || body.length < 10) events.push({ type: 'blank', text: `body length ${body.length}` });
    const links = await page.locator('a[href]').evaluateAll(els => els.map(a => ({ text: a.textContent?.trim() || '', href: a.href })));
    for (const link of links) {
      if (link.href.startsWith(base)) {
        const u = new URL(link.href);
        if (u.pathname.startsWith('/api/')) continue;
      }
    }
  } catch (err) {
    events.push({ type: 'exception', text: err.message });
  }
  summary.push({ route, title, body: body.slice(0, 160).replace(/\s+/g, ' '), events });
  await page.close();
}

console.log(JSON.stringify(summary, null, 2));
await browser.close();
