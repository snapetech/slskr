# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: search.spec.ts >> search >> local_search_returns_fixture_hits
- Location: e2e/search.spec.ts:25:7

# Error details

```
Error: expect(received).toBeGreaterThan(expected)

Expected: > 0
Received:   0
```

# Test source

```ts
  1   | import { NODES, shouldLaunchNodes } from './env';
  2   | import { MultiPeerHarness } from './harness/MultiPeerHarness';
  3   | import { clickNav, login, waitForHealth } from './helpers';
  4   | import { T } from './selectors';
  5   | import { expect, test } from '@playwright/test';
  6   |
  7   | test.describe('search', () => {
  8   |   let harness: MultiPeerHarness | null = null;
  9   |
  10  |   test.beforeAll(async () => {
  11  |     if (shouldLaunchNodes()) {
  12  |       harness = new MultiPeerHarness();
  13  |       await harness.startNode('A', 'test-data/slskr-test-fixtures/music', {
  14  |         noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
  15  |       });
  16  |     }
  17  |   });
  18  |
  19  |   test.afterAll(async () => {
  20  |     if (harness) {
  21  |       await harness.stopAll();
  22  |     }
  23  |   });
  24  |
  25  |   test('local_search_returns_fixture_hits', async ({ page, request }) => {
  26  |     const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
  27  |     await waitForHealth(request, nodeA.baseUrl);
  28  |     await login(page, nodeA);
  29  |
  30  |     // Navigate directly to search page (route is /searches)
  31  |     await page.goto(`${nodeA.baseUrl}/searches`, {
  32  |       timeout: 10_000,
  33  |       waitUntil: 'domcontentloaded',
  34  |     });
  35  |
  36  |     // Wait for search UI - try multiple selectors
  37  |     const searchInput = page
  38  |       .getByTestId(T.searchInput)
  39  |       .or(page.locator('input[placeholder*="search" i]'))
  40  |       .first();
  41  |     await expect(searchInput).toBeVisible({ timeout: 10_000 });
  42  |
  43  |     // When SLSKR_TEST_NO_CONNECT=true the input is disabled; assert and pass
  44  |     const isEnabled = await searchInput.isEnabled().catch(() => false);
  45  |     if (!isEnabled) {
  46  |       await expect(searchInput).toBeDisabled();
  47  |       return;
  48  |     }
  49  |
  50  |     // Music fixture is test-data/slskr-test-fixtures/music/open_goldberg/ (cover.jpg)
  51  |     await searchInput.fill('cover');
  52  |
  53  |     // Wait for search request/response (POST /api/v0/searches or compatibility search)
  54  |     const searchResponse = page
  55  |       .waitForResponse(
  56  |         (resp) =>
  57  |           (resp.url().includes('/api/v0/search') ||
  58  |             resp.url().includes('/searches')) &&
  59  |           (resp.status() === 200 || resp.status() === 201),
  60  |         { timeout: 15_000 },
  61  |       )
  62  |       .catch(() => null);
  63  |
  64  |     await searchInput.press('Enter');
  65  |     await searchResponse; // Wait for API call
  66  |
  67  |     // Result cards use className "result-card" (Search/Response.jsx). Wait for navigation to
  68  |     // /searches/<id> and for at least one card to appear (search detail loads asynchronously).
  69  |     const results = page.locator(
  70  |       '[data-testid*="search-result"], [data-testid*="result-item"], .result-card, .search-result, .result-item',
  71  |     );
  72  |     await expect(results.first()).toBeVisible({ timeout: 20_000 }).catch(() => null);
  73  |     const count = await results.count();
  74  |
  75  |     // If no results in UI, check API response directly (GET podcore content search)
  76  |     if (count === 0) {
  77  |       const apiResponse = await request.get(
  78  |         `${nodeA.baseUrl}/api/v0/podcore/content/search?query=cover`,
  79  |         { failOnStatusCode: false },
  80  |       );
  81  |       if (apiResponse.ok()) {
  82  |         const body = await apiResponse.json().catch(() => ({}));
  83  |         if (Array.isArray(body) && body.length > 0) {
  84  |           return; // Search works, UI might not be showing yet
  85  |         }
  86  |       }
  87  |     }
  88  |
> 89  |     expect(count).toBeGreaterThan(0);
      |                   ^ Error: expect(received).toBeGreaterThan(expected)
  90  |   });
  91  |
  92  |   test('no_connect_disables_soulseek_provider_gracefully', async ({
  93  |     page,
  94  |     request,
  95  |   }) => {
  96  |     const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
  97  |     await waitForHealth(request, nodeA.baseUrl);
  98  |     await login(page, nodeA);
  99  |
  100 |     // Navigate to search page (route is /searches)
  101 |     await page.goto(`${nodeA.baseUrl}/searches`, {
  102 |       timeout: 10_000,
  103 |       waitUntil: 'domcontentloaded',
  104 |     });
  105 |
  106 |     // Verify page loads without crashing
  107 |     await expect(page.locator('body')).toBeVisible({ timeout: 3_000 });
  108 |
  109 |     // If no_connect is enabled, verify graceful handling
  110 |     if (process.env.SLSKR_TEST_NO_CONNECT === 'true') {
  111 |       // Check connection status if it exists
  112 |       const connectionStatus = page.getByTestId(T.connectionStatus);
  113 |       if ((await connectionStatus.count()) > 0) {
  114 |         await expect(connectionStatus).toBeVisible({ timeout: 5_000 });
  115 |       }
  116 |
  117 |       // Verify search still works (local search should work even without Soulseek)
  118 |       const searchInput = page
  119 |         .getByTestId(T.searchInput)
  120 |         .or(page.locator('input[placeholder*="search" i]'))
  121 |         .first();
  122 |       if ((await searchInput.count()) > 0) {
  123 |         await searchInput.fill('test');
  124 |         await searchInput.press('Enter');
  125 |         // Should not crash - local search should work
  126 |         await page.waitForTimeout(1_000);
  127 |       }
  128 |     }
  129 |   });
  130 | });
  131 |
```
