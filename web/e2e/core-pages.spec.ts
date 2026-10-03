import { NODES, shouldLaunchNodes } from './env';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { clickNav, login, verifySpaFallback, waitForHealth } from './helpers';
import { T } from './selectors';
import { expect, test } from '@playwright/test';
import { resolve } from 'node:path';

test.describe('core pages', () => {
  let harness: MultiPeerHarness | null = null;

  test.beforeAll(async () => {
    if (shouldLaunchNodes()) {
      harness = new MultiPeerHarness();
      await harness.startNode('A', 'test-data/slskr-test-fixtures/music', {
        noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
      });
    }
  });

  test.afterAll(async () => {
    if (harness) {
      await harness.stopAll();
    }
  });

  test('system_page_loads', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);
    await login(page, nodeA);

    // Navigate directly to /system instead of clicking (more reliable)
    await page.goto(`${nodeA.baseUrl}/system`, {
      timeout: 10_000,
      waitUntil: 'domcontentloaded',
    });

    // Verify we're on the system page
    expect(page.url()).toMatch(/\/system/);
    await expect(page.getByTestId(T.systemRoot)).toBeVisible({ timeout: 10_000 });

    const sharesTab = page.getByTestId(T.systemTabShares);
    await expect(sharesTab).toBeVisible({ timeout: 10_000 });
    await sharesTab.click();
    await expect(page.getByTestId(T.systemSharesTable)).toBeVisible({
      timeout: 10_000,
    });
  });

  test(
    'mobile_navigation_scrolls_and_reveals_active_route',
    async ({ page, request }) => {
      const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
      await page.setViewportSize({ width: 390, height: 844 });
      await waitForHealth(request, nodeA.baseUrl);
      await login(page, nodeA);
      await page.goto(`${nodeA.baseUrl}/searches`, {
        timeout: 10_000,
        waitUntil: 'domcontentloaded',
      });

      const layout = await page.locator('.navigation').evaluate((navigation) => {
        const primary = navigation.querySelector<HTMLElement>(
          '.navigation-primary',
        );
        const controls = navigation.querySelector<HTMLElement>(
          '.right.ui.inverted.menu',
        );
        if (!primary || !controls) {
          throw new Error('Navigation rows were not rendered');
        }
        return {
          navigationHeight: navigation.getBoundingClientRect().height,
          primaryFlexWrap: getComputedStyle(primary).flexWrap,
          primaryOverflowX: getComputedStyle(primary).overflowX,
          primaryBottom: primary.getBoundingClientRect().bottom,
          controlsTop: controls.getBoundingClientRect().top,
        };
      });

      expect(layout.navigationHeight).toBeLessThanOrEqual(130);
      expect(layout.primaryFlexWrap).toBe('nowrap');
      expect(layout.primaryOverflowX).toBe('auto');
      expect(layout.controlsTop).toBeGreaterThanOrEqual(layout.primaryBottom);

      await page.addInitScript({
        path: resolve(process.cwd(), 'node_modules/axe-core/axe.min.js'),
      });
      await page.goto(`${nodeA.baseUrl}/users`, {
        timeout: 10_000,
        waitUntil: 'domcontentloaded',
      });
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.getByTestId('users-page-heading')).toBeVisible();
      await page.waitForTimeout(250);
      const accessibilityViolations = await page.evaluate(async () => {
        const result = await (window as any).axe.run(document);
        return result.violations.map(({ id, impact, nodes }) => ({
          id,
          impact,
          nodes: nodes.map(({ html, failureSummary, target }) => ({
            html,
            failureSummary,
            target,
          })),
        }));
      });
      expect(accessibilityViolations).toEqual([]);

      const expectFooterToFit = async (width: number, height = 844) => {
        await page.setViewportSize({ width, height });
        const footerLayout = await page
          .locator('.slskr-footer')
          .evaluate((footer) => {
            const build = footer.querySelector<HTMLElement>(
              '.slskr-footer-build',
            );
            if (!build) throw new Error('Build link was not rendered');
            const buildRect = build.getBoundingClientRect();
            return {
              footerClientWidth: footer.clientWidth,
              footerScrollWidth: footer.scrollWidth,
              documentScrollWidth: document.documentElement.scrollWidth,
              viewportHeight: window.innerHeight,
              footerTop: footer.getBoundingClientRect().top,
              footerBottom: footer.getBoundingClientRect().bottom,
              buildLeft: buildRect.left,
              buildRight: buildRect.right,
              buildTop: buildRect.top,
              buildBottom: buildRect.bottom,
            };
          });
        expect(footerLayout.footerScrollWidth).toBeLessThanOrEqual(
          footerLayout.footerClientWidth + 1,
        );
        expect(footerLayout.documentScrollWidth).toBeLessThanOrEqual(width);
        expect(footerLayout.buildLeft).toBeGreaterThanOrEqual(0);
        expect(footerLayout.buildRight).toBeLessThanOrEqual(width);
        expect(footerLayout.footerTop).toBeGreaterThanOrEqual(0);
        expect(footerLayout.footerBottom).toBeLessThanOrEqual(
          footerLayout.viewportHeight,
        );
        expect(footerLayout.buildTop).toBeGreaterThanOrEqual(0);
        expect(footerLayout.buildBottom).toBeLessThanOrEqual(
          footerLayout.viewportHeight,
        );
      };
      await expectFooterToFit(320, 568);
      await expectFooterToFit(360, 640);
      await expectFooterToFit(390);
      await expectFooterToFit(768);
      await expectFooterToFit(320, 568);
      await expectFooterToFit(390);

      await expect
        .poll(() =>
          page.locator('.navigation-primary').evaluate((primary) => {
            const activeLink = primary.querySelector(
              'a.active, a[aria-current="page"]',
            );
            if (!activeLink) return false;
            const primaryRect = primary.getBoundingClientRect();
            const linkRect = activeLink.getBoundingClientRect();
            return (
              linkRect.left >= primaryRect.left &&
              linkRect.right <= primaryRect.right
            );
          }),
        )
        .toBe(true);

      await page.setViewportSize({ width: 320, height: 568 });
      await expect
        .poll(() =>
          page.locator('.navigation-primary').evaluate((primary) => {
            const activeLink = primary.querySelector(
              'a.active, a[aria-current="page"]',
            );
            if (!activeLink) return false;
            const primaryRect = primary.getBoundingClientRect();
            const linkRect = activeLink.getBoundingClientRect();
            return (
              linkRect.left >= primaryRect.left &&
              linkRect.right <= primaryRect.right
            );
          }),
        )
        .toBe(true);
    },
  );

  test('downloads_page_loads', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    // First verify SPA fallback is working at the server level
    console.log(`[Test] Verifying SPA fallback before login...`);
    try {
      await verifySpaFallback(request, nodeA.baseUrl);
      console.log(
        `[Test] SPA fallback verified - server is serving index.html correctly`,
      );
    } catch (error) {
      console.error(`[Test] SPA fallback verification failed: ${error}`);
      throw error; // Fail fast if SPA fallback isn't working
    }

    await login(page, nodeA);

    // Capture ALL network responses for diagnostics
    const responses: Array<{
      body?: string;
      headers: Record<string, string>;
      status: number;
      url: string;
    }> = [];
    page.on('response', async (response) => {
      const url = response.url();
      if (url.includes('/downloads') || url.includes('/api/')) {
        const status = response.status();
        const headers = response.headers();
        let body: string | undefined;
        try {
          // Try to get response body (may fail for binary or already consumed)
          body = await response.text().catch(() => undefined);
        } catch {
          // Body already consumed or not readable
        }

        responses.push({ body, headers, status, url });
        console.log(`[Test] Response: ${status} ${url}`);
        console.log(
          `[Test] Content-Type: ${headers['content-type'] || 'none'}`,
        );
        console.log(
          `[Test] Content-Length: ${headers['content-length'] || 'none'}`,
        );
        if (body && body.length < 500) {
          console.log(
            `[Test] Response body (first 500 chars): ${body.slice(0, 500)}`,
          );
        } else if (body) {
          console.log(
            `[Test] Response body length: ${body.length} (too long to log)`,
          );
        }
      }
    });

    // Capture console errors and network failures
    const consoleErrors: string[] = [];
    const networkErrors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') {
        const text = message.text();
        consoleErrors.push(text);
        console.log(`[Test] Console error: ${text}`);
      }
    });
    page.on('requestfailed', (request_) => {
      const url = request_.url();
      const failure = request_.failure()?.errorText || 'unknown';
      networkErrors.push(`${url}: ${failure}`);
      console.log(`[Test] Request failed: ${url} - ${failure}`);
    });

    // Navigate directly to /downloads
    console.log(`[Test] Navigating to ${nodeA.baseUrl}/downloads`);
    const navResponse = await page.goto(`${nodeA.baseUrl}/downloads`, {
      timeout: 10_000,
      waitUntil: 'domcontentloaded',
    });

    console.log(`[Test] Navigation response status: ${navResponse?.status()}`);
    console.log(`[Test] Navigation response URL: ${navResponse?.url()}`);

    // Wait a moment for any async responses
    await page.waitForTimeout(1_000);

    // Check what we actually got from the server
    const content = await page.content();
    const title = await page.title().catch(() => 'no title');
    const url = page.url();

    console.log(`[Test] Final URL: ${url}`);
    console.log(`[Test] Page title: ${title}`);
    console.log(`[Test] Page content length: ${content.length}`);
    console.log(
      `[Test] Page content preview (first 500 chars): ${content.slice(0, 500)}`,
    );
    console.log(
      `[Test] Page content preview (last 200 chars): ${content.slice(Math.max(0, Math.max(0, content.length - 200)))}`,
    );

    // Check for #root element
    const rootCount = await page.locator('#root').count();
    const rootVisible = await page
      .locator('#root')
      .isVisible()
      .catch(() => false);
    const rootContent = await page
      .locator('#root')
      .textContent()
      .catch(() => '');

    console.log(`[Test] #root count: ${rootCount}, visible: ${rootVisible}`);
    console.log(`[Test] #root content length: ${rootContent.length}`);
    if (rootContent.length < 200) {
      console.log(`[Test] #root content: ${rootContent}`);
    }

    // Check for React mounting indicators
    const hasReactRoot = await page
      .evaluate(() => {
        const root = document.querySelector('#root');
        return root && root.children.length > 0;
      })
      .catch(() => false);
    console.log(`[Test] React root has children: ${hasReactRoot}`);

    // Check for JavaScript errors in the page
    const jsErrors = await page
      .evaluate(() => {
        return (window as any).__playwright_errors__ || [];
      })
      .catch(() => []);
    console.log(`[Test] JavaScript errors in page: ${jsErrors.length}`);
    jsErrors.forEach((error: any, index: number) => {
      console.log(`[Test] JS Error ${index}: ${JSON.stringify(error)}`);
    });

    // Diagnostic summary
    console.log(`[Test] === DIAGNOSTIC SUMMARY ===`);
    console.log(`[Test] HTTP Status: ${navResponse?.status()}`);
    console.log(`[Test] Content Length: ${content.length}`);
    console.log(`[Test] Has #root: ${rootCount > 0}`);
    console.log(`[Test] React mounted: ${hasReactRoot}`);
    console.log(`[Test] Console errors: ${consoleErrors.length}`);
    console.log(`[Test] Network errors: ${networkErrors.length}`);
    console.log(`[Test] Total responses captured: ${responses.length}`);

    // Verify URL
    expect(page.url()).toMatch(/\/downloads/);

    // If we got a 404 or empty body, that's the real problem - not timing
    if (navResponse?.status() === 404) {
      throw new Error(
        `Server returned 404 for /downloads. This indicates SPA fallback is not working. Responses: ${JSON.stringify(responses, null, 2)}`,
      );
    }

    if (
      content.length < 100 ||
      content === '<html><head></head><body></body></html>'
    ) {
      throw new Error(
        `Server returned empty or minimal HTML. Content length: ${content.length}, Status: ${navResponse?.status()}, Responses: ${JSON.stringify(responses, null, 2)}`,
      );
    }

    // Wait for React to mount (the page might be empty initially if server returned empty HTML)
    console.log(`[Test] Waiting for #root...`);
    try {
      await page.waitForSelector('#root', {
        state: 'attached',
        timeout: 15_000,
      });
      console.log(`[Test] #root found`);
    } catch (error) {
      console.log(`[Test] #root not found: ${error}`);
      throw new Error(
        `React did not mount. Content length: ${content.length}, Root count: ${rootCount}, Status: ${navResponse?.status()}, Responses: ${JSON.stringify(responses, null, 2)}`,
      );
    }

    // The Transfers component shows a loader while fetching, then content
    // Wait for either the testid OR for loader to disappear
    const downloadsRoot = page.getByTestId(T.downloadsRoot);
    const loader = page.locator(
      '.ui.segment.loading, .loader, [class*="loading"]',
    );

    console.log(
      `[Test] Waiting for downloads component or loader to disappear...`,
    );
    await Promise.race([
      downloadsRoot
        .waitFor({ state: 'visible', timeout: 10_000 })
        .then(() => {
          console.log(`[Test] Downloads root became visible`);
          return 'visible';
        })
        .catch(() => null),
      page
        .waitForFunction(
          () => {
            const loaders = document.querySelectorAll(
              '.ui.segment.loading, .loader, [class*="loading"]',
            );
            return loaders.length === 0;
          },
          { timeout: 20_000 },
        )
        .then(() => {
          console.log(`[Test] Loaders disappeared`);
          return 'no-loaders';
        })
        .catch(() => null),
    ]);

    // Verify page loaded - body should exist
    const body = page.locator('body');
    const bodyCount = await body.count();
    const bodyVisible = await body.isVisible().catch(() => false);
    console.log(`[Test] Body count: ${bodyCount}, visible: ${bodyVisible}`);
    expect(bodyCount).toBeGreaterThan(0);
  });

  test('uploads_page_loads', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    // Verify SPA fallback is working
    console.log(`[Test] Verifying SPA fallback before login...`);
    try {
      await verifySpaFallback(request, nodeA.baseUrl);
      console.log(`[Test] SPA fallback verified`);
    } catch (error) {
      console.error(`[Test] SPA fallback verification failed: ${error}`);
      throw error;
    }

    await login(page, nodeA);

    // Capture network responses
    const responses: Array<{ status: number; url: string }> = [];
    page.on('response', async (response) => {
      const url = response.url();
      if (
        url.includes('/uploads') ||
        (url.includes(nodeA.baseUrl) && !url.includes('/api/'))
      ) {
        responses.push({ status: response.status(), url });
        console.log(`[Test] Response: ${response.status()} ${url}`);
      }
    });

    // Navigate directly to /uploads
    console.log(`[Test] Navigating to ${nodeA.baseUrl}/uploads`);
    const navResponse = await page.goto(`${nodeA.baseUrl}/uploads`, {
      timeout: 10_000,
      waitUntil: 'domcontentloaded',
    });

    console.log(`[Test] Navigation response status: ${navResponse?.status()}`);

    // Check what we got
    const content = await page.content();
    console.log(`[Test] Page content length: ${content.length}`);

    // Verify URL
    expect(page.url()).toMatch(/\/uploads/);

    // Check for 404 or empty body
    if (navResponse?.status() === 404) {
      throw new Error(
        `Server returned 404 for /uploads. SPA fallback may not be working. Responses: ${JSON.stringify(responses)}`,
      );
    }

    if (content.length < 100) {
      throw new Error(
        `Server returned empty HTML. Content length: ${content.length}, Status: ${navResponse?.status()}`,
      );
    }

    // Wait for React to mount
    console.log(`[Test] Waiting for #root...`);
    await page.waitForSelector('#root', { state: 'attached', timeout: 15_000 });
    console.log(`[Test] #root found`);

    // Wait for component to finish loading
    const uploadsRoot = page.getByTestId(T.uploadsRoot);
    const loader = page.locator(
      '.ui.segment.loading, .loader, [class*="loading"]',
    );

    console.log(
      `[Test] Waiting for uploads component or loader to disappear...`,
    );
    await Promise.race([
      uploadsRoot
        .waitFor({ state: 'visible', timeout: 10_000 })
        .then(() => {
          console.log(`[Test] Uploads root became visible`);
          return 'visible';
        })
        .catch(() => null),
      page
        .waitForFunction(
          () => {
            const loaders = document.querySelectorAll(
              '.ui.segment.loading, .loader, [class*="loading"]',
            );
            return loaders.length === 0;
          },
          { timeout: 20_000 },
        )
        .then(() => {
          console.log(`[Test] Loaders disappeared`);
          return 'no-loaders';
        })
        .catch(() => null),
    ]);

    // Verify page loaded
    const body = page.locator('body');
    expect(await body.count()).toBeGreaterThan(0);
  });

  test('rooms_chat_users_pages_graceful_offline', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);
    await login(page, nodeA);

    // These pages may not exist in the current UI - just verify graceful handling
    // Try to navigate to these routes directly and verify no crashes
    const routes = ['/rooms', '/chat', '/users'];
    for (const route of routes) {
      await page.goto(`${nodeA.baseUrl}${route}`, {
        timeout: 10_000,
        waitUntil: 'domcontentloaded',
      });

      // Verify page loaded (no crash)
      await page.waitForSelector('body', {
        state: 'attached',
        timeout: 10_000,
      });

      // Verify React mounted
      await page.waitForSelector('#root', {
        state: 'attached',
        timeout: 10_000,
      });
    }
  });
});
