import { NODES, shouldLaunchNodes } from './env';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { goto, login, waitForHealth } from './helpers';
import { T } from './selectors';
import { expect, test } from '@playwright/test';

test.describe('smoke/auth', () => {
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

  test('health_and_login', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    // Capture console logs and network errors for debugging
    page.on('console', (message) => {
      if (message.type() === 'error') {
        console.log(`[Browser Console Error] ${message.text()}`);
      }
    });

    page.on('pageerror', (error) => {
      console.log(`[Page Error] ${error.message}`);
    });

    page.on('response', (response) => {
      if (!response.ok() && response.url().includes('/api/')) {
        console.log(`[API Error] ${response.status()} ${response.url()}`);
      }
    });

    await login(page, nodeA);

    await expect(page.getByTestId(T.navSystem)).toBeVisible({ timeout: 20_000 });
    const token = await page.evaluate(() =>
      sessionStorage.getItem('slskr-token') || localStorage.getItem('slskr-token'),
    );
    expect(token).toBeTruthy();

    // Take a screenshot for debugging
    await page.screenshot({
      fullPage: true,
      path: 'test-results/login-after.png',
    });

    // Log the current URL and page content for debugging
    console.log(`[Debug] Current URL: ${page.url()}`);
    const bodyText = await page.locator('body').textContent();
    console.log(`[Debug] Body text preview: ${bodyText?.slice(0, 200)}`);
  });

  test('route_guard', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    // Capture console logs for debugging
    page.on('console', (message) => {
      if (message.type() === 'error') {
        console.log(`[Browser Console Error] ${message.text()}`);
      }
    });

    // Ensure we're not logged in - clear any existing tokens
    await page.goto(nodeA.baseUrl, {
      timeout: 30_000,
      waitUntil: 'networkidle',
    });
    await page.evaluate(() => {
      sessionStorage.removeItem('slskr-token');
      localStorage.removeItem('slskr-token');
    });

    // Navigate to protected route - wait for page to fully load
    await page.goto(`${nodeA.baseUrl}/system`, {
      timeout: 30_000,
      waitUntil: 'networkidle',
    });

    await expect(page.getByTestId(T.loginUsername)).toBeVisible({ timeout: 20_000 });
    await expect(page.locator('[data-testid^="nav-"]')).toHaveCount(0);
  });

  test('logout', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    // Capture console logs for debugging
    page.on('console', (message) => {
      if (message.type() === 'error') {
        console.log(`[Browser Console Error] ${message.text()}`);
      }
    });

    await login(page, nodeA);

    // Click logout Menu.Item (this opens a modal)
    // The logout button is a Menu.Item with data-testid="logout"
    let logoutMenuItem;
    try {
      logoutMenuItem = page.getByTestId(T.logout);
      await expect(logoutMenuItem).toBeVisible({ timeout: 15_000 });
    } catch {
      // Fallback: try finding by text in menu
      logoutMenuItem = page.locator('.ui.menu .item:has-text("Log Out")');
      await expect(logoutMenuItem.first()).toBeVisible({ timeout: 15_000 });
    }

    await logoutMenuItem.click();

    // Wait for modal to appear and confirm logout. Scoped to the modal
    // itself — the trigger Menu.Item is also role="button" named "Log Out"
    // and stays in the DOM while the modal is open.
    const confirmButton = page
      .locator('.ui.modal')
      .getByRole('button', { name: /^log out$/i });
    await expect(confirmButton).toBeVisible({ timeout: 10_000 });
    await confirmButton.click();

    // Should return to login - wait for navigation and login form
    await page.waitForTimeout(1_000); // Give time for logout to process

    // Wait for either URL change or login form to appear
    await Promise.race([
      page
        .waitForURL(
          (url) => url.includes('/login') || !url.includes('/system'),
          { timeout: 10_000 },
        )
        .catch(() => {}),
      page
        .waitForSelector(`[data-testid="${T.loginUsername}"]`, {
          timeout: 10_000,
        })
        .catch(() => {}),
      page
        .waitForSelector('input[placeholder*="Username" i]', {
          timeout: 10_000,
        })
        .catch(() => {}),
    ]);

    await expect(page.getByTestId(T.loginUsername)).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.locator('[data-testid^="nav-"]')).toHaveCount(0);
    const tokenAfterLogout = await page.evaluate(() =>
      sessionStorage.getItem('slskr-token') || localStorage.getItem('slskr-token'),
    );
    expect(tokenAfterLogout).toBeNull();
  });
});
