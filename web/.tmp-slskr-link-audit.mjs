import { chromium } from 'playwright';

const base = process.env.UI_BASE || 'http://127.0.0.1:3001';
const seedRoutes = [
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
const safeButton = /\b(refresh|reload|load|close|cancel|clear filters|reset filters)\b/i;
const unsafeButton = /\b(delete|remove|save|create|add|join|leave|connect|disconnect|start|stop|run|download|upload|publish|rescan|resolve|import|export|ban|unban|acknowledge|send)\b/i;

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const routes = new Set(seedRoutes);
const summary = [];

const attachWatchers = (page, events) => {
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
};

for (const route of seedRoutes) {
  const page = await context.newPage();
  const events = [];
  attachWatchers(page, events);
  await page.goto(`${base}${route}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await page.waitForTimeout(1500);
  const links = await page.locator('a[href]').evaluateAll((anchors, origin) => anchors
    .map(a => ({ href: a.href, text: a.textContent?.trim() || '' }))
    .filter(link => link.href.startsWith(origin))
    .map(link => {
      const url = new URL(link.href);
      return { path: `${url.pathname}${url.search}${url.hash}`, text: link.text };
    })
    .filter(link => !link.path.startsWith('/api/') && link.path !== '#'), base);
  for (const link of links) routes.add(link.path);

  const buttons = await page.getByRole('button').evaluateAll(buttons => buttons.map((button, index) => ({
    index,
    text: button.innerText || button.getAttribute('aria-label') || button.getAttribute('title') || '',
  })));
  const clicked = [];
  for (const button of buttons) {
    const label = button.text.trim();
    if (!safeButton.test(label) || unsafeButton.test(label)) continue;
    const before = events.length;
    await page.getByRole('button').nth(button.index).click({ timeout: 2000 }).catch(error => {
      events.push({ type: 'click', text: `${label}: ${error.message}` });
    });
    await page.waitForTimeout(500);
    clicked.push({ label, newEvents: events.length - before });
  }
  summary.push({ route, links: links.length, clicked, events });
  await page.close();
}

for (const route of [...routes]) {
  const page = await context.newPage();
  const events = [];
  attachWatchers(page, events);
  const response = await page.goto(`${base}${route}`, { waitUntil: 'domcontentloaded', timeout: 15000 }).catch(error => {
    events.push({ type: 'navigation', text: error.message });
    return null;
  });
  if (!response || response.status() >= 400) {
    events.push({ type: 'navigation', text: `${response?.status()} ${base}${route}` });
  }
  await page.waitForTimeout(800);
  const body = (await page.locator('body').innerText({ timeout: 5000 }).catch(() => '')).trim();
  if (body.length < 10) events.push({ type: 'blank', text: `body length ${body.length}` });
  summary.push({ route, discoveredNavigation: true, events });
  await page.close();
}

console.log(JSON.stringify(summary.filter(item => item.events.length > 0), null, 2));
console.error(JSON.stringify({
  routesVisited: routes.size,
  seedRoutes: seedRoutes.length,
  issueCount: summary.reduce((count, item) => count + item.events.length, 0),
}, null, 2));
await browser.close();
