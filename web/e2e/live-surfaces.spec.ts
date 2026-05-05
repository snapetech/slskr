import { NODES, shouldLaunchNodes } from './env';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { login, waitForHealth } from './helpers';
import { T } from './selectors';
import { expect, test } from '@playwright/test';

const routes = [
  { name: 'search', path: '/searches', pattern: /search/i },
  { name: 'downloads', path: '/downloads', pattern: /download|transfer/i },
  { name: 'uploads', path: '/uploads', pattern: /upload|transfer/i },
  { name: 'messages', path: '/messages', pattern: /message|conversation|chat/i },
  { name: 'rooms', path: '/rooms', pattern: /room/i },
  { name: 'browse', path: '/browse', pattern: /browse|library|share/i },
  { name: 'system', path: '/system', pattern: /system|server|share/i },
];

test.describe('live app surfaces', () => {
  let harness: MultiPeerHarness | null = null;

  test.beforeAll(async () => {
    if (shouldLaunchNodes()) {
      harness = new MultiPeerHarness();
      await harness.startNode('A', 'test-data/slskdn-test-fixtures/music', {
        noConnect: process.env.SLSKDN_TEST_NO_CONNECT === 'true',
      });
    }
  });

  test.afterAll(async () => {
    if (harness) {
      await harness.stopAll();
    }
  });

  test('daemon_backed_routes_and_player_controls_do_not_regress', async ({
    page,
    request,
  }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);
    await login(page, nodeA);

    const consoleErrors: string[] = [];
    page.on('console', (message) => {
      if (message.type() === 'error') {
        const text = message.text();
        if (
          !text.includes('/api/events/ws') &&
          !text.includes('WebSocket connection error')
        ) {
          consoleErrors.push(text);
        }
      }
    });

    for (const route of routes) {
      const response = await page.goto(`${nodeA.baseUrl}${route.path}`, {
        timeout: 10_000,
        waitUntil: 'domcontentloaded',
      });
      expect(response?.status(), `${route.name} route status`).not.toBe(404);
      await expect(page.locator('#root')).toBeVisible({ timeout: 10_000 });
      await expect(page.locator('body')).toContainText(route.pattern, {
        timeout: 10_000,
      });
    }

    await expect(page.getByTestId(T.playerTogglePlayback)).toBeVisible({
      timeout: 10_000,
    });
    await expect(page.getByTestId(T.playerToggleMute)).toBeVisible();
    await expect(page.getByTestId(T.playerVisualTile)).toBeVisible();

    if (await page.getByTestId(T.playerToggleMute).isEnabled()) {
      await page.getByTestId(T.playerToggleMute).click();
    }

    if (await page.getByTestId(T.playerOpenQueue).isEnabled()) {
      await page.getByTestId(T.playerOpenQueue).click();
      await expect(page.locator('.player-queue-manager, .player-queue')).toBeVisible({
        timeout: 5_000,
      });
    }

    if (await page.getByTestId(T.playerToggleEq).isEnabled()) {
      await page.getByTestId(T.playerToggleEq).click();
      await expect(page.locator('.player-panel-eq')).toBeVisible({
        timeout: 5_000,
      });
    }

    await page.getByTestId(T.playerCollapse).click();
    await expect(page.getByTestId(T.playerExpand)).toBeVisible({
      timeout: 5_000,
    });

    const runtimeErrors = await page.evaluate(
      () => (window as any).__playwright_errors__ || [],
    );
    expect(runtimeErrors, 'captured runtime errors').toEqual([]);
    expect(consoleErrors, 'console errors').toEqual([]);
  });
});
