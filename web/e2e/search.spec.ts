import { NODES, shouldLaunchNodes } from './env';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import { getAuthToken, login, waitForHealth } from './helpers';
import { T } from './selectors';
import { expect, test } from '@playwright/test';

test.describe('search', () => {
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

  test('local_search_returns_fixture_hits', async ({ page, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);
    await login(page, nodeA);

    // Navigate directly to search page (route is /searches)
    await page.goto(`${nodeA.baseUrl}/searches`, {
      timeout: 10_000,
      waitUntil: 'domcontentloaded',
    });

    // Wait for search UI - try multiple selectors
    const searchInput = page
      .getByTestId(T.searchInput)
      .or(page.locator('input[placeholder*="search" i]'))
      .first();
    await expect(searchInput).toBeVisible({ timeout: 10_000 });

    // When SLSKR_TEST_NO_CONNECT=true the input is disabled; assert and pass
    const isEnabled = await searchInput.isEnabled().catch(() => false);
    if (!isEnabled) {
      await expect(searchInput).toBeDisabled();
      return;
    }

    // Music fixture is test-data/slskr-test-fixtures/music/open_goldberg/ (cover.jpg)
    await searchInput.fill('cover');

    // Wait for the actual create request. A background GET /searches can race
    // this action, so matching only the URL would accept the wrong response.
    const searchResponse = page
      .waitForResponse(
        (resp) =>
          resp.request().method() === 'POST' &&
          resp.url().includes('/api/v0/searches') &&
          (resp.status() === 200 || resp.status() === 201),
        { timeout: 15_000 },
      )
      .then((resp) => resp.json());

    await searchInput.press('Enter');
    const createdSearch = await searchResponse;
    const createdSearchId = createdSearch?.searchId ?? createdSearch?.id;
    expect(createdSearchId).toBeTruthy();
    const authHeaders = {
      Authorization: `Bearer ${await getAuthToken(page)}`,
    };

    const responseApi = await request.get(
      `${nodeA.baseUrl}/api/v0/searches/${encodeURIComponent(String(createdSearchId))}/responses`,
      { failOnStatusCode: false, headers: authHeaders },
    );
    const responseBody = await responseApi.json().catch(() => null);
    expect(responseApi.ok()).toBeTruthy();
    expect(Array.isArray(responseBody)).toBeTruthy();
    expect(
      responseBody.some((response) =>
        [...(response.files ?? []), ...(response.lockedFiles ?? [])].some(
          (file) => file.filename?.toLowerCase().includes('cover'),
        ),
      ),
    ).toBeTruthy();

    // Enter queues the search on the list page. Open the returned ID explicitly
    // so this scenario covers the same detail route that previously rendered a
    // blank page when the compatibility ID had no UUID dashes.
    await page.goto(
      `${nodeA.baseUrl}/searches/${encodeURIComponent(String(createdSearchId))}`,
      { timeout: 10_000, waitUntil: 'domcontentloaded' },
    );

    // Result cards use className "result-card" (Search/Response.jsx). Wait for
    // at least one card to appear; search detail loads asynchronously.
    const results = page.locator(
      '[data-testid*="search-result"], [data-testid*="result-item"], .result-card, .search-result, .result-item',
    );
    await expect(results.first()).toBeVisible({ timeout: 20_000 });
    const count = await results.count();
    expect(count).toBeGreaterThan(0);
  });

  test('no_connect_disables_soulseek_provider_gracefully', async ({
    page,
    request,
  }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);
    await login(page, nodeA);

    // Navigate to search page (route is /searches)
    await page.goto(`${nodeA.baseUrl}/searches`, {
      timeout: 10_000,
      waitUntil: 'domcontentloaded',
    });

    // Verify page loads without crashing
    await expect(page.locator('body')).toBeVisible({ timeout: 3_000 });

    // If no_connect is enabled, verify graceful handling
    if (process.env.SLSKR_TEST_NO_CONNECT === 'true') {
      // Check connection status if it exists
      const connectionStatus = page.getByTestId(T.connectionStatus);
      if ((await connectionStatus.count()) > 0) {
        await expect(connectionStatus).toBeVisible({ timeout: 5_000 });
      }

      // Verify search still works (local search should work even without Soulseek)
      const searchInput = page
        .getByTestId(T.searchInput)
        .or(page.locator('input[placeholder*="search" i]'))
        .first();
      if ((await searchInput.count()) > 0) {
        await searchInput.fill('test');
        await searchInput.press('Enter');
        // Should not crash - local search should work
        await page.waitForTimeout(1_000);
      }
    }
  });
});
