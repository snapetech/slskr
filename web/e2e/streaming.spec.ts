import { holdMovieStream, incomingMovieStream } from './fixtures/ticketed-share';
import { NODES, shouldLaunchNodes } from './env';
import { hasMediaFixture } from './fixtures/ensure-fixtures';
import { MultiPeerHarness } from './harness/MultiPeerHarness';
import {
  announceShareGrant,
  clickNav,
  getAuthToken,
  login,
  waitForHealth,
  waitForLibraryItem,
  waitForShareGrantById,
} from './helpers';
import { T } from './selectors';
import { expect, test } from '@playwright/test';

const hasDownloadedMedia = hasMediaFixture('movie/sintel_512kb_stereo.mp4');

test.describe('streaming', () => {
  test.describe.configure({ mode: 'serial' });
  test.skip(!hasDownloadedMedia, 'Streaming E2E requires the pinned Sintel movie fixture');

  let harness: MultiPeerHarness | null = null;
  const groupName = 'E2E Crew';
  const collectionTitle = 'E2E Streaming Test';
  let sharedGrantId: string | null = null;
  let sharedCollectionId: string | null = null;
  let ownerAuthToken: string | null = null;
  let recipientAuthToken: string | null = null;

  test.beforeAll(async () => {
    if (shouldLaunchNodes()) {
      harness = new MultiPeerHarness();
      await harness.startNode('A', 'test-data/slskr-test-fixtures/movie', {
        noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
      });
      await harness.startNode('B', 'test-data/slskr-test-fixtures/book', {
        noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
      });
    }
  });

  test.afterAll(async () => {
    if (harness) {
      await harness.stopAll();
    }
  });

  test('recipient_streams_item_with_range', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    const nodeB = harness ? harness.getNode('B').nodeCfg : NODES.B;
    await waitForHealth(request, nodeA.baseUrl);
    await waitForHealth(request, nodeB.baseUrl);

    const contextA = await browser.newContext();
    const contextB = await browser.newContext();
    const pageA = await contextA.newPage();
    const pageB = await contextB.newPage();

    await login(pageA, nodeA);
    await login(pageB, nodeB);

    // Ensure group and collection exist (reuse from multippeer-sharing tests)
    await clickNav(pageA, T.navGroups);
    const existingGroupRow = pageA.getByTestId(T.groupRow(groupName));
    if ((await existingGroupRow.count()) === 0) {
      await pageA.getByTestId(T.groupsCreate).click();
      await pageA.waitForSelector(`[data-testid="${T.groupsNameInput}"]`, {
        timeout: 5_000,
      });
      await pageA
        .getByTestId(T.groupsNameInput)
        .locator('input')
        .fill(groupName);
      await pageA.getByTestId(T.groupsCreateSubmit).click();
      await expect(pageA.getByTestId(T.groupRow(groupName))).toBeVisible({
        timeout: 5_000,
      });
    }

    const setupToken = await getAuthToken(pageA);
    const groups = await request.get(`${nodeA.baseUrl}/api/v0/sharegroups`, {
      headers: { Authorization: `Bearer ${setupToken}` },
    });
    expect(groups.status()).toBe(200);
    const group = (await groups.json()).find((entry: { name?: string }) => entry.name === groupName);
    expect(group).toBeTruthy();
    const member = await request.post(`${nodeA.baseUrl}/api/v0/sharegroups/${group.id}/members`, {
      data: { username: nodeB.username }, headers: { Authorization: `Bearer ${setupToken}` },
    });
    expect([200, 201]).toContain(member.status());

    // Create collection and share (similar to multippeer-sharing test)
    await clickNav(pageA, T.navCollections);
    await pageA.waitForSelector('[data-testid="collections-root"]', {
      timeout: 10_000,
    });

    const existingCollectionRow = pageA.getByTestId(
      T.collectionRow(collectionTitle),
    );
    if ((await existingCollectionRow.count()) === 0) {
      await pageA.getByTestId(T.collectionsCreate).click();
      await pageA.waitForSelector(
        `[data-testid="${T.collectionsTypeSelect}"]`,
        { timeout: 5_000 },
      );
      await pageA.getByTestId(T.collectionsTypeSelect).click();
      await pageA.getByRole('option', { name: /playlist/i }).click();
      await pageA
        .getByTestId(T.collectionsTitleInput)
        .locator('input')
        .fill(collectionTitle);

      const createCollectionResponse = pageA.waitForResponse(
        (response) =>
          response.url().includes('/api/v0/collections') &&
          response.request().method() === 'POST',
        { timeout: 5_000 },
      );
      await pageA.getByTestId(T.collectionsCreateSubmit).click();
      const createCollectionResult = await createCollectionResponse;
      if (createCollectionResult.status() !== 201) {
        const body = await createCollectionResult.text();
        throw new Error(
          `Create collection failed: ${createCollectionResult.status()} ${body}`,
        );
      }

      await expect(
        pageA.getByTestId(T.collectionRow(collectionTitle)),
      ).toBeVisible({ timeout: 5_000 });
    }

    await pageA.getByTestId(T.collectionRow(collectionTitle)).click();
    await pageA.waitForTimeout(200); // Reduced from 500ms

    // Add item
    const addItemButton = pageA.getByTestId(T.collectionAddItem);
    if ((await addItemButton.count()) > 0) {
      await addItemButton.click();
      const item = await waitForLibraryItem(pageA, 'sintel_512kb_stereo');
      await pageA
        .getByTestId(T.collectionItemPicker)
        .locator('input')
        .fill(item.contentId);
      await pageA.getByTestId(T.collectionAddItemSubmit).click();
      await pageA.waitForTimeout(500); // Reduced from 1000ms
    }

    // Share with stream enabled
    const shareCreate = pageA.getByTestId(T.shareCreate);
    await expect(shareCreate).toBeVisible({ timeout: 5_000 });
    await shareCreate.click();
    const audiencePicker = pageA.getByTestId(T.shareAudiencePicker);
    await expect(audiencePicker).toBeVisible({ timeout: 5_000 });
    await audiencePicker.click();
    const groupOption = pageA.getByRole('option', {
      name: new RegExp(groupName, 'i'),
    });
    if ((await groupOption.count()) === 0) {
      throw new Error(
        'No share groups found in picker. Ensure group creation ran.',
      );
    }

    await groupOption.first().click();

    await pageA.getByTestId(T.sharePolicyStream).check();
    await pageA.getByTestId(T.sharePolicyDownload).check();

    const createShareResponse = pageA.waitForResponse(
      (response) =>
        response.url().includes('/api/v0/share-grants') &&
        response.request().method() === 'POST',
      { timeout: 5_000 },
    );
    await pageA.getByTestId(T.shareCreateSubmit).click();
    const createShareResult = await createShareResponse;
    let createShareBody;
    try {
      createShareBody = await createShareResult.json();
    } catch {
      createShareBody = await createShareResult.text();
    }

    if (createShareResult.status() !== 201) {
      throw new Error(
        `Create share failed: ${createShareResult.status()} ${typeof createShareBody === 'string' ? createShareBody : JSON.stringify(createShareBody)}`,
      );
    }

    if (!createShareBody?.id) {
      throw new Error('Create share response missing id.');
    }

    const ownerToken = await getAuthToken(pageA);
    const recipientToken = await getAuthToken(pageB);
    await announceShareGrant({
      owner: nodeA,
      ownerToken,
      recipient: nodeB,
      recipientToken,
      request,
      shareGrantId: createShareBody.id,
    });
    sharedGrantId = createShareBody.id;
    sharedCollectionId = createShareBody.collectionId;
    ownerAuthToken = ownerToken;
    recipientAuthToken = recipientToken;

    // Wait for cross-node discovery (reduced - announceShareGrant should make this faster)
    await pageB.waitForTimeout(2_000); // Reduced from 5000ms

    // Node B tries to stream
    await clickNav(pageB, T.navSharedWithMe);
    await pageB.waitForTimeout(1_000); // Reduced from 2000ms

    // Poll for the share to appear
    let shareFound = false;
    let streamUrl: string | null = null;
    for (let index = 0; index < 20; index++) {
      const shareRow = pageB
        .getByTestId(T.incomingShareRow(collectionTitle))
        .first();
      if ((await shareRow.count()) > 0) {
        shareFound = true;
        await shareRow.getByTestId(T.incomingShareOpen).click();
        await expect(pageB.getByTestId(T.sharedManifest)).toBeVisible({
          timeout: 15_000,
        });

        streamUrl = await incomingMovieStream({
          request, recipient: nodeB, recipientToken, owner: nodeA, title: collectionTitle,
        });

        break;
      }

      await pageB.waitForTimeout(500); // Reduced from 1000ms
    }

    expect(shareFound).toBe(true);
    if (!streamUrl) {
      throw new Error('No streamUrl found in manifest for streaming test.');
    }

    // Make a Range request (simulating stream)
    const normalized = streamUrl
      .replace('http://localhost:', 'http://127.0.0.1:')
      .replace('https://localhost:', 'https://127.0.0.1:');
    const fullStreamUrl = normalized.startsWith('http')
      ? normalized
      : `${nodeB.baseUrl}${normalized}`;

    const rangeResponse = await request.get(fullStreamUrl, {
      failOnStatusCode: false,
      headers: { Range: 'bytes=0-1' },
    });

    expect(rangeResponse.status()).toBe(206);
    expect(rangeResponse.headers()['content-type']).toBe('video/mp4');
    expect(rangeResponse.headers()['content-range']).toBe('bytes 0-1/77410288');
    expect(await rangeResponse.body()).toEqual(Buffer.from([0, 0]));

    await contextA.close();
    await contextB.close();
  });

  test('seek_works_with_range_requests', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    const nodeB = harness ? harness.getNode('B').nodeCfg : NODES.B;
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await login(page, nodeB);
      const recipientToken = await getAuthToken(page);
      const streamUrl = await incomingMovieStream({
        request, recipient: nodeB, recipientToken, owner: nodeA, title: collectionTitle,
      });
      for (const range of ['bytes=0-15', 'bytes=65536-65551', 'bytes=-16']) {
        const response = await request.get(streamUrl, { headers: { Range: range } });
        expect(response.status()).toBe(206);
        expect(response.headers()['content-type']).toMatch(/video/u);
        expect(response.headers()['content-range']).toMatch(/^bytes \d+-\d+\/77410288$/u);
        expect((await response.body()).length).toBe(16);
      }
      const noTicket = new URL(streamUrl);
      noTicket.search = '';
      expect((await request.get(noTicket.toString())).status()).toBe(401);
      noTicket.search = '?ticket=invalid-ticket';
      expect((await request.get(noTicket.toString())).status()).toBe(401);
      noTicket.search = '?token=forbidden-query-token';
      expect((await request.get(noTicket.toString())).status()).toBe(400);
    } finally {
      await context.close();
    }
  });

  test('concurrency_limit_blocks_excess_streams', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    const nodeB = harness ? harness.getNode('B').nodeCfg : NODES.B;
    const context = await browser.newContext();
    let held: Awaited<ReturnType<typeof holdMovieStream>> | undefined;
    try {
      const page = await context.newPage();
      await login(page, nodeA);
      const ownerToken = await getAuthToken(page);
      const auth = { Authorization: `Bearer ${ownerToken}` };
      const item = await waitForLibraryItem(page, 'sintel_512kb_stereo');
      const title = `E2E Concurrency ${Date.now()}`;
      const collectionResponse = await request.post(`${nodeA.baseUrl}/api/v0/collections`, {
        headers: auth, data: { title, type: 'Playlist' },
      });
      expect(collectionResponse.status()).toBe(201);
      const collection = await collectionResponse.json();
      const added = await request.post(`${nodeA.baseUrl}/api/v0/collections/${collection.id}/items`, {
        headers: auth, data: { contentId: item.contentId, mediaKind: item.mediaKind },
      });
      expect([200, 201]).toContain(added.status());
      const created = await request.post(`${nodeA.baseUrl}/api/v0/share-grants`, {
        headers: auth, data: { collectionId: collection.id, username: nodeB.username,
          allowDownload: true, allowStream: true, allowReshare: false, maxConcurrentStreams: 1 },
      });
      expect(created.status()).toBe(201);
      const grant = await created.json();
      expect(grant.maxConcurrentStreams).toBe(1);
      await login(page, nodeB);
      const recipientToken = await getAuthToken(page);
      await announceShareGrant({ request, owner: nodeA, ownerToken, recipient: nodeB,
        recipientToken, shareGrantId: grant.id });
      const streamUrl = await incomingMovieStream({ request, recipient: nodeB,
        recipientToken, owner: nodeA, title });
      held = await holdMovieStream(streamUrl);
      expect(held.status).toBe(206);
      const rejected = await request.get(streamUrl, { headers: { Range: 'bytes=0-1' }, timeout: 10_000 });
      expect(rejected.status()).toBe(429);
      await held.close();
      held = undefined;
      await expect.poll(async () => {
        const response = await request.get(streamUrl, { headers: { Range: 'bytes=0-1' }, timeout: 10_000 });
        return response.status();
      }).toBe(206);
    } finally {
      await held?.close();
      await context.close();
    }
  });
});
