import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { incomingMovieStream } from './fixtures/ticketed-share';
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

test.describe.configure({ mode: 'serial' });
const hasDownloadedMedia = hasMediaFixture('movie/sintel_512kb_stereo.mp4');

test.describe('multi-peer sharing', () => {
  test.skip(
    !hasDownloadedMedia,
    'Multi-peer sharing E2E requires the pinned Sintel movie fixture',
  );

  let harness: MultiPeerHarness | null = null;
  const groupName = 'E2E Crew';
  const collectionTitle = 'E2E Playlist';
  let sharedGrantId: string | null = null;

  test.beforeAll(async () => {
    if (shouldLaunchNodes()) {
      harness = new MultiPeerHarness();
      // Node A: shares movie/ + book/
      await harness.startNode(
        'A',
        [
          'test-data/slskr-test-fixtures/movie',
          'test-data/slskr-test-fixtures/book',
        ],
        {
          noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
        },
      );
      // Node B: shares music/ + tv/
      await harness.startNode(
        'B',
        [
          'test-data/slskr-test-fixtures/music',
          'test-data/slskr-test-fixtures/tv',
        ],
        {
          noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
        },
      );
      // Node C: recipient-only (no shares) with A's existing single-port mesh
      // endpoint pinned from its locally generated certificate.
      const ownerMeshPeer = await harness
        .getNode('A')
        .trustedMeshPeerConfig();
      await harness.startNode('C', [], {
        noConnect: process.env.SLSKR_TEST_NO_CONNECT === 'true',
        trustedMeshPeers: [ownerMeshPeer],
      });
    }
  });

  test.afterAll(async () => {
    if (harness) {
      await harness.stopAll();
    }
  });

  test('invite_add_friend', async ({ browser, request }, testInfo) => {
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

    // Diagnostic: Capture JS/runtime errors BEFORE navigation
    pageA.on('pageerror', (error) =>
      console.error('[Contacts Test] pageerror:', error),
    );
    pageA.on('console', (message) => {
      if (message.type() === 'error')
        console.error('[Contacts Test] console.error:', message.text());
    });

    // Navigate to contacts page and wait for it to load
    console.log('[Contacts Test] Navigating to contacts page...');
    const targetUrl = `${nodeA.baseUrl}/contacts`;
    console.log('[Contacts Test] Target URL:', targetUrl);
    const contactsResponse = pageA.waitForResponse(
      (response) => new URL(response.url()).pathname === '/api/v0/contacts' && response.request().method() === 'GET',
      { timeout: 10_000 },
    );
    // Attach a rejection handler immediately; the awaited assertion below owns failure.
    void contactsResponse.catch(() => {});
    await pageA.goto(targetUrl, { timeout: 10_000, waitUntil: 'networkidle' });
    console.log('[Contacts Test] Navigation complete, URL:', pageA.url());

    // Diagnostic: Compare browser location vs app location (memory history check)
    const loc = await pageA.evaluate(() => ({
      href: location.href,
      pathname: location.pathname,
    }));
    const appLoc = await pageA.evaluate(() => {
      if ((window as any).__APP_HISTORY__) {
        return (window as any).__APP_HISTORY__.location.pathname;
      }

      if ((window as any).__APP_LOCATION__) {
        return (window as any).__APP_LOCATION__.pathname;
      }

      return null;
    });
    console.log(
      '[Contacts Test] Browser location:',
      JSON.stringify(loc, null, 2),
    );
    console.log('[Contacts Test] App location/history:', appLoc);

    // Check if URL changed (redirect happened)
    const finalUrl = pageA.url();
    if (!finalUrl.includes('/contacts')) {
      console.error(
        `[Contacts Test] ERROR: Redirected away from /contacts! Final URL: ${finalUrl}`,
      );
    }

    // Check what's actually on the page
    const bodyContent = await pageA.locator('body').innerText();
    console.log(
      '[Contacts Test] Body text (first 500 chars):',
      bodyContent.slice(0, 500),
    );

    // Check which component is actually rendering
    const hasSearchElements = await pageA
      .locator('input[placeholder*="Search"], [data-testid*="search"]')
      .count();
    const hasContactsElements = await pageA
      .locator('[data-testid="contacts-root"], [data-testid*="contact"]')
      .count();
    console.log('[Contacts Test] Search elements count:', hasSearchElements);
    console.log(
      '[Contacts Test] Contacts elements count:',
      hasContactsElements,
    );

    // Check React component tree if possible
    const reactRoot = await pageA.evaluate(() => {
      const root = document.querySelector('#root');
      if (!root) return null;
      const firstChild = root.firstElementChild;
      return {
        firstChildClass: firstChild?.className,
        firstChildTag: firstChild?.tagName,
        firstChildText: firstChild?.textContent?.slice(0, 100),
        rootTag: root.tagName,
      };
    });
    console.log(
      '[Contacts Test] React root info:',
      JSON.stringify(reactRoot, null, 2),
    );

    // Check if we're still on login page (route guard)
    const loginForm = await pageA
      .locator(
        '[data-testid="login-username"], input[placeholder*="Username" i]',
      )
      .count();
    console.log('[Contacts Test] Login form count:', loginForm);
    if (loginForm > 0) {
      console.error(
        '[Contacts Test] ERROR: Still on login page - route guard may be blocking!',
      );
    }

    // Check for any React error boundaries or error messages
    const errorElements = await pageA
      .locator('[class*="error"], [class*="Error"], [data-testid*="error"]')
      .count();
    console.log('[Contacts Test] Error elements count:', errorElements);

    // Check what React Router thinks the current route is
    const currentRoute = await pageA.evaluate(() => {
      // Try to find React Router state or current pathname
      return {
        hash: window.location.hash,
        pathname: window.location.pathname,
        search: window.location.search,
        urlBase: (window as any).urlBase || 'not set',
      };
    });
    console.log(
      '[Contacts Test] Current route info:',
      JSON.stringify(currentRoute, null, 2),
    );

    // Wait for contacts root to appear (ensures component mounted)
    console.log('[Contacts Test] Waiting for contacts-root...');
    try {
      await pageA.waitForSelector('[data-testid="contacts-root"]', {
        timeout: 10_000,
      });
      console.log('[Contacts Test] contacts-root found - component mounted');
    } catch (error) {
      console.error('[Contacts Test] ERROR: contacts-root not found!');
      // Dump all data-testid elements to see what's actually rendered
      const allTestIds = await pageA.evaluate(() => {
        const elements = document.querySelectorAll<HTMLElement>('[data-testid]');
        return Array.from(elements).map((element) => ({
          tag: element.tagName,
          testid: element.dataset.testid,
          visible: (element as HTMLElement).offsetParent !== null,
        }));
      });
      console.log(
        '[Contacts Test] All data-testid elements on page:',
        JSON.stringify(allTestIds, null, 2),
      );
      throw error;
    }

    const contacts = await contactsResponse;
    expect(contacts.status()).toBe(200);
    expect(contacts.headers()['content-type']).toContain('application/json');
    expect(await contacts.json()).toBeInstanceOf(Array);

    // Diagnostic: Check current state
    const tid = T.contactsCreateInvite;
    const count = await pageA.locator(`[data-testid="${tid}"]`).count();
    console.log('[Contacts Test] data-testid count', tid, count);

    // If present, dump visibility diagnostics
    if (count > 0) {
      const diag = await pageA
        .locator(`[data-testid="${tid}"]`)
        .first()
        .evaluate((element) => {
          const cs = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return {
            disabled: (element as any).disabled ?? null,
            display: cs.display,
            inDocument: document.contains(element),
            opacity: cs.opacity,
            rect: { h: rect.height, w: rect.width, x: rect.x, y: rect.y },
            tag: element.tagName,
            visibility: cs.visibility,
          };
        });
      console.log(
        '[Contacts Test] button diag:',
        JSON.stringify(diag, null, 2),
      );
    } else {
      console.error(
        `[Contacts Test] ERROR: Button with data-testid="${tid}" not found in DOM!`,
      );
    }

    // Screenshot and body snippet for debugging
    await pageA.screenshot({
      fullPage: true,
      path: testInfo.outputPath('contacts-debug.png'),
    });
    const bodyText = await pageA.locator('body').innerText();
    console.log(
      '[Contacts Test] body snippet (first 800 chars):',
      bodyText.slice(0, 800),
    );

    // Wait for create invite button (appears in header - always visible)
    const createInviteButton = pageA.getByTestId(T.contactsCreateInvite);
    await expect(createInviteButton.first()).toBeVisible({ timeout: 5_000 });

    await createInviteButton.click();

    // Wait for invite modal and get invite link
    const inviteOutput = pageA.getByTestId(T.contactsInviteOutput);
    await expect(inviteOutput).toBeVisible({ timeout: 5_000 });
    const invite = await inviteOutput.inputValue();
    expect(invite.length).toBeGreaterThan(20);

    // Node B adds friend
    await pageB.goto(`${nodeB.baseUrl}/contacts`, {
      timeout: 5_000,
      waitUntil: 'networkidle',
    });

    const addFriendButton = pageB.getByTestId(T.contactsAddFriend);
    await expect(addFriendButton).toBeVisible({ timeout: 5_000 });
    await addFriendButton.click();

    // Fill invite form
    const inviteInput = pageB.getByTestId(T.contactsAddInviteInput);
    await expect(inviteInput).toBeVisible({ timeout: 3_000 });
    await inviteInput.fill(invite);

    const nicknameInput = pageB.getByTestId(T.contactsContactNickname);
    await expect(nicknameInput).toBeVisible({ timeout: 3_000 });
    await nicknameInput.fill('nodeA');

    await pageB.getByTestId(T.contactsAddInviteSubmit).click();

    // Contact row should appear after adding
    const contactRow = pageB.getByTestId(T.contactsRow('nodeA'));
    await expect(contactRow).toBeVisible({ timeout: 5_000 });

    await contextA.close();
    await contextB.close();
  });

  test('create_group_add_member', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    const contextA = await browser.newContext();
    const pageA = await contextA.newPage();
    await login(pageA, nodeA);

    await clickNav(pageA, T.navGroups);
    await pageA.getByTestId(T.groupsCreate).click();

    // Wait for create group modal
    await pageA.waitForSelector(`[data-testid="${T.groupsNameInput}"]`, {
      timeout: 5_000,
    });
    // Semantic UI wraps inputs in divs - select the actual input element
    await pageA.getByTestId(T.groupsNameInput).locator('input').fill(groupName);
    await pageA.getByTestId(T.groupsCreateSubmit).click();

    await expect(pageA.getByTestId(T.groupRow(groupName))).toBeVisible({
      timeout: 5_000,
    });

    // Add member - button is in the table row
    // Note: For this test to work, nodeA needs to have nodeB as a contact
    // The first test (invite_add_friend) has nodeB add nodeA, so we need bidirectional
    // For now, skip if no contacts available
    const addMemberButton = pageA
      .getByTestId(T.groupRow(groupName))
      .locator(`[data-testid="${T.groupAddMember}"]`)
      .first();
    await expect(addMemberButton).toBeVisible({ timeout: 5_000 });

    // Click and wait for modal to appear (check for modal header first, then picker)
    await addMemberButton.click();

    // Wait for modal to open - check for modal header text or the picker
    // Semantic UI modals might take a moment to animate in
    try {
      await pageA.waitForSelector('text=Add Member to', { timeout: 5_000 });
      console.log('[Test] Modal header found');
    } catch {
      console.log(
        '[Test] Modal header not found, checking for picker directly',
      );
    }

    // Check if contacts are available - modal shows different UI if no contacts
    // If no contacts, it shows an input field instead of the picker dropdown
    const picker = pageA.getByTestId(T.groupMemberPicker);
    const pickerVisible = await picker.isVisible().catch(() => false);

    if (!pickerVisible) {
      // No contacts available - add by Soulseek username (legacy)
      const userInput = pageA
        .locator('.ui.modal')
        .locator('input[placeholder*="username" i]')
        .first();
      await expect(userInput).toBeVisible({ timeout: 5_000 });
      await userInput.fill('nodeB');
      await pageA.getByTestId(T.groupMemberAddSubmit).click();
      await expect(userInput).not.toBeVisible({ timeout: 5_000 });
    } else {
      // Contacts available - use the dropdown picker
      await picker.click();

      // Wait for dropdown options to appear, then select first available contact
      await pageA.getByRole('option').first().click({ timeout: 5_000 });

      await pageA.getByTestId(T.groupMemberAddSubmit).click();

      // Wait for modal to close (member was added)
      await expect(picker).not.toBeVisible({ timeout: 5_000 });
    }

    await contextA.close();
  });

  test('create_collection_share_to_group', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    await waitForHealth(request, nodeA.baseUrl);

    const contextA = await browser.newContext();
    const pageA = await contextA.newPage();
    await login(pageA, nodeA);

    // Ensure the share group exists (so this test can run standalone)
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

    // The recipient must be a real share-group member before the authenticated
    // mesh backfill can authorize the grant. Ensure this in both isolated and
    // full serialized runs rather than relying on the earlier nodeB UI case.
    const nodeC = harness ? harness.getNode('C').nodeCfg : NODES.C;
    const ownerToken = await getAuthToken(pageA);
    const ownerHeaders = { Authorization: `Bearer ${ownerToken}` };
    const groupsResponse = await request.get(
      `${nodeA.baseUrl}/api/v0/sharegroups`,
      { headers: ownerHeaders },
    );
    expect(groupsResponse.status()).toBe(200);
    const groups = await groupsResponse.json();
    const recipientGroup = Array.isArray(groups)
      ? groups.find((group: any) => group?.name === groupName)
      : null;
    expect(recipientGroup?.id).toBeTruthy();
    const membersResponse = await request.get(
      `${nodeA.baseUrl}/api/v0/sharegroups/${recipientGroup.id}/members`,
      { headers: ownerHeaders },
    );
    expect(membersResponse.status()).toBe(200);
    const members = await membersResponse.json();
    if (
      !Array.isArray(members) ||
      !members.some(
        (member: any) =>
          member?.username?.toLowerCase() === nodeC.username.toLowerCase(),
      )
    ) {
      const addRecipient = await request.post(
        `${nodeA.baseUrl}/api/v0/sharegroups/${recipientGroup.id}/members`,
        {
          data: { username: nodeC.username },
          headers: ownerHeaders,
          failOnStatusCode: false,
        },
      );
      expect([200, 201]).toContain(addRecipient.status());
    }

    // Navigate to collections page directly
    await pageA.goto(`${nodeA.baseUrl}/collections`, {
      timeout: 10_000,
      waitUntil: 'networkidle',
    });

    // Wait a moment for React Router to process
    await pageA.waitForTimeout(500); // Reduced from 1000ms // Reduced from 2000ms

    // Diagnostic: Check if route matched
    const routeMatched = await pageA.evaluate(
      () => (window as any).routeMatchedCollections || false,
    );
    console.log('[Collections Test] Route matched flag:', routeMatched);

    // Diagnostic: Check router state
    const loc = await pageA.evaluate(() => location.pathname);
    console.log('[Collections Test] window.location.pathname =', loc);
    const urlBase = await pageA.evaluate(
      () => (window as any).urlBase || 'not set',
    );
    console.log('[Collections Test] window.urlBase =', urlBase);

    const collectionsRootCount = await pageA
      .locator('[data-testid="collections-root"]')
      .count();
    if (!collectionsRootCount && !routeMatched && loc === '/searches') {
      const title = await pageA.title();
      throw new Error(
        `Stale WebUI bundle detected: /collections route missing (title=${title}, url=${pageA.url()}). ` +
          'Run `npm run build` and re-run e2e with harness-launched nodes.',
      );
    }

    // Check for route miss (via window flag or DOM element) - check multiple times as redirect might clear it
    const routeMissPath = await pageA.evaluate(() => {
      // Check both flags
      return (
        (window as any).routeMissPath ||
        (window as any).routeMissElement ||
        null
      );
    });
    const routeMissText = await pageA.evaluate(() => {
      const element = document.querySelector('[data-testid="route-miss"]');
      return element ? element.textContent : null;
    });

    console.log('[Collections Test] Route miss path:', routeMissPath);
    console.log('[Collections Test] Route miss text:', routeMissText);

    if (routeMissPath || routeMissText) {
      console.error(
        '[Collections Test] ROUTE MISS DETECTED:',
        routeMissPath || routeMissText,
      );
      throw new Error(`Route miss detected: ${routeMissPath || routeMissText}`);
    }

    // If route didn't match, that's the problem
    if (!routeMatched && loc === '/searches') {
      console.error(
        '[Collections Test] ERROR: Route did not match - redirected to /searches',
      );
      // Dump all route-related info
      const routeInfo = await pageA.evaluate(() => ({
        href: location.href,
        pathname: location.pathname,
        routeMatched: (window as any).routeMatchedCollections || false,
        routeMiss: (window as any).routeMissPath || null,
        routeMissElement: (window as any).routeMissElement || null,
        urlBase: (window as any).urlBase || 'not set',
      }));
      console.error(
        '[Collections Test] Route info:',
        JSON.stringify(routeInfo, null, 2),
      );
      throw new Error(
        `Route /collections did not match. Route miss: ${routeMissPath || routeMissText || 'unknown'}`,
      );
    }

    // Wait for collections page to load
    await pageA.waitForSelector('[data-testid="collections-root"]', {
      timeout: 10_000,
    });

    // Create collection
    const createButton = pageA.getByTestId(T.collectionsCreate);
    await expect(createButton).toBeVisible({ timeout: 5_000 });
    await createButton.click();

    await pageA.waitForSelector(`[data-testid="${T.collectionsTypeSelect}"]`, {
      timeout: 5_000,
    });
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
    await pageA.getByTestId(T.collectionRow(collectionTitle)).click();

    // Add real fixture items using library search
    // The collection row click should open the detail view
    await pageA.waitForTimeout(200); // Wait for selection (reduced from 500ms)

    // Click Add Item button
    const addItemButton = pageA.getByTestId('collection-add-item');
    await expect(addItemButton).toBeVisible({ timeout: 5_000 });
    await addItemButton.click();

    // Search for sintel (movie fixture) and add by contentId
    const searchInput = pageA.getByTestId('collection-item-search-input');
    await expect(searchInput).toBeVisible({ timeout: 5_000 });
    const sintelItem = await waitForLibraryItem(pageA, 'sintel_512kb_stereo');
    await searchInput.locator('input').fill(sintelItem.contentId);

    // Add the item
    await pageA.getByTestId('collection-add-item-submit').click();
    await pageA.waitForTimeout(500); // Reduced from 1000ms

    // Add second item: treasure (book fixture)
    await addItemButton.click();
    const treasureItem = await waitForLibraryItem(pageA, 'treasure');
    await searchInput.locator('input').fill(treasureItem.contentId);
    await pageA.getByTestId('collection-add-item-submit').click();
    await pageA.waitForTimeout(500); // Reduced from 1000ms

    // Verify items were added
    await expect(pageA.getByTestId('collection-items-table')).toBeVisible({
      timeout: 5_000,
    });

    // Share it
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

    const collectionId =
      createShareBody.collectionId || createShareBody.collection_id;
    expect(collectionId).toBeTruthy();
    const ownerSharesResponse = await request.get(
      `${nodeA.baseUrl}/api/v0/share-grants`,
      { headers: ownerHeaders },
    );
    expect(ownerSharesResponse.status()).toBe(200);
    const ownerShares = await ownerSharesResponse.json();
    const recipientGrant = Array.isArray(ownerShares)
      ? ownerShares.find(
          (grant: any) =>
            (grant?.collectionId || grant?.collection_id) === collectionId &&
            grant?.username?.toLowerCase() === nodeC.username.toLowerCase(),
        )
      : null;
    expect(recipientGrant?.id).toBeTruthy();
    sharedGrantId = recipientGrant.id;

    const contextC = await browser.newContext();
    const pageC = await contextC.newPage();
    await login(pageC, nodeC);
    const recipientToken = await getAuthToken(pageC);
    await announceShareGrant({
      owner: nodeA,
      ownerToken,
      recipient: nodeC,
      recipientToken,
      request,
      shareGrantId: recipientGrant.id,
    });
    await contextC.close();

    await expect(pageA.getByTestId(T.sharesList)).toContainText(
      collectionTitle,
      { timeout: 5_000 },
    );

    await contextA.close();
  });

  test('recipient_sees_shared_manifest', async ({ browser, request }) => {
    const nodeC = harness ? harness.getNode('C').nodeCfg : NODES.C;
    await waitForHealth(request, nodeC.baseUrl);

    const contextC = await browser.newContext();
    const pageC = await contextC.newPage();
    await login(pageC, nodeC);

    await clickNav(pageC, T.navSharedWithMe);

    let rowFound = false;
    for (let index = 0; index < 20; index++) {
      const row = pageC
        .getByTestId(`incoming-share-row-${collectionTitle}`)
        .first();
      if ((await row.count()) > 0) {
        rowFound = true;
        await row.getByTestId('incoming-share-open').click();
        break;
      }

      await pageC.waitForTimeout(500); // Reduced from 1000ms
    }

    expect(rowFound).toBe(true);

    await expect(pageC.getByTestId('shared-manifest')).toBeVisible({
      timeout: 15_000,
    });

    // Verify real fixture items appear (sintel or treasure)
    const manifestContent = await pageC
      .getByTestId('shared-manifest')
      .textContent();
    expect(manifestContent).toMatch(/sha256:/i);

    await contextC.close();
  });

  test('recipient_streams_video', async ({ browser, request }) => {
    const nodeA = harness ? harness.getNode('A').nodeCfg : NODES.A;
    const nodeC = harness ? harness.getNode('C').nodeCfg : NODES.C;
    await waitForHealth(request, nodeA.baseUrl);
    await waitForHealth(request, nodeC.baseUrl);

    const contextA = await browser.newContext();
    const contextC = await browser.newContext();
    const pageA = await contextA.newPage();
    const pageC = await contextC.newPage();
    await login(pageA, nodeA);
    await login(pageC, nodeC);

    // Ensure share is announced to nodeC
    const ownerToken = await getAuthToken(pageA);
    let recipientToken = await getAuthToken(pageC);

    // Get or create share grant
    let shareGrantId = sharedGrantId;
    let shareOverride: any | undefined;
    if (shareGrantId) {
      const checkRes = await request.get(
        `${nodeA.baseUrl}/api/v0/share-grants/${shareGrantId}`,
        {
          failOnStatusCode: false,
          headers: { Authorization: `Bearer ${ownerToken}` },
        },
      );
      if (checkRes.status() === 404) {
        shareGrantId = null;
        sharedGrantId = null;
      } else if (checkRes.ok()) {
        try {
          shareOverride = await checkRes.json();
        } catch {
          shareOverride = undefined;
        }
      }
    }
    if (!shareGrantId) {
      // Create share grant if it doesn't exist
      const groupsRes = await request.get(
        `${nodeA.baseUrl}/api/v0/sharegroups`,
        {
          failOnStatusCode: false,
          headers: { Authorization: `Bearer ${ownerToken}` },
        },
      );
      if (!groupsRes.ok()) {
        throw new Error(`Failed to load share groups: ${groupsRes.status()}`);
      }

      const groups = await groupsRes.json();
      let group = Array.isArray(groups)
        ? groups.find((g: any) => g?.name === groupName)
        : null;
      if (!group) {
        const createGroupRes = await request.post(
          `${nodeA.baseUrl}/api/v0/sharegroups`,
          {
            data: { name: groupName },
            headers: { Authorization: `Bearer ${ownerToken}` },
          },
        );
        if (!createGroupRes.ok()) {
          throw new Error(
            `Failed to create share group: ${createGroupRes.status()}`,
          );
        }

        group = await createGroupRes.json();
      }

      const collectionsRes = await request.get(
        `${nodeA.baseUrl}/api/v0/collections`,
        { headers: { Authorization: `Bearer ${ownerToken}` } },
      );
      const collections = await collectionsRes.json();
      let collection = Array.isArray(collections)
        ? collections.find((c: any) => c?.title === collectionTitle)
        : null;
      if (!collection) {
        const createCollectionRes = await request.post(
          `${nodeA.baseUrl}/api/v0/collections`,
          {
            data: { title: collectionTitle, type: 'Playlist' },
            headers: { Authorization: `Bearer ${ownerToken}` },
          },
        );
        if (!createCollectionRes.ok()) {
          throw new Error(
            `Failed to create collection: ${createCollectionRes.status()}`,
          );
        }

        collection = await createCollectionRes.json();
      }

      // Ensure collection has at least one video item so streaming works when this test
      // runs in isolation (it normally relies on earlier tests to populate items).
      const existingItemsRes = await request.get(
        `${nodeA.baseUrl}/api/v0/collections/${collection.id}/items`,
        { headers: { Authorization: `Bearer ${ownerToken}` } },
      );
      if (!existingItemsRes.ok()) {
        throw new Error(
          `Failed to load collection items: ${existingItemsRes.status()}`,
        );
      }

      const existingItems = await existingItemsRes.json();
      if (!Array.isArray(existingItems) || existingItems.length === 0) {
        // Deterministic fixture: test-data/slskr-test-fixtures/book/treasure_island_pg120.txt
        // Use the library endpoint so the share repository has a matching ContentId.
        const libraryRes = await request.get(
          `${nodeA.baseUrl}/api/v0/library/items?query=treasure_island_pg120.txt&limit=1`,
          { headers: { Authorization: `Bearer ${ownerToken}` } },
        );
        if (!libraryRes.ok()) {
          throw new Error(
            `Failed to load library items: ${libraryRes.status()}`,
          );
        }

        const libraryPayload = await libraryRes.json();
        const libraryItems = Array.isArray(libraryPayload?.items)
          ? libraryPayload.items
          : [];
        const firstItem = libraryItems[0];
        const contentId = firstItem?.contentId || firstItem?.content_id;
        const mediaKind = firstItem?.mediaKind || firstItem?.media_kind;
        if (!contentId) {
          throw new Error('No library items available for streaming share.');
        }

        const addItemRes = await request.post(
          `${nodeA.baseUrl}/api/v0/collections/${collection.id}/items`,
          {
            data: {
              contentId,
              mediaKind,
            },
            headers: { Authorization: `Bearer ${ownerToken}` },
          },
        );
        if (!addItemRes.ok()) {
          const body = await addItemRes.text();
          throw new Error(
            `Failed to add collection item: ${addItemRes.status()} ${body}`,
          );
        }
      }

      const createShareRes = await request.post(
        `${nodeA.baseUrl}/api/v0/share-grants`,
        {
          data: {
            allowDownload: true,
            allowReshare: false,
            allowStream: true,
            audienceId: group.id,
            audienceType: 'ShareGroup',
            collectionId: collection.id,
          },
          headers: { Authorization: `Bearer ${ownerToken}` },
        },
      );
      if (!createShareRes.ok()) {
        throw new Error(
          `Failed to create share grant: ${createShareRes.status()}`,
        );
      }

      const share = await createShareRes.json();
      if (!share?.id) {
        throw new Error('Create share grant response missing id.');
      }

      shareGrantId = share.id;
      sharedGrantId = share.id;
      shareOverride = share;
    }

    if (!shareGrantId) throw new Error('Movie share grant is missing');
    // Announce share to nodeC
    await announceShareGrant({
      owner: nodeA,
      ownerToken,
      recipient: nodeC,
      recipientToken,
      request,
      shareGrantId,
      shareOverride,
    });

    // Wait for share to be available via API (more reliable than UI polling)
    recipientToken = await getAuthToken(pageC);
    const shareAvailable = await waitForShareGrantById({
      baseUrl: nodeC.baseUrl,
      request,
      shareGrantId,
      timeoutMs: 30_000,
      token: recipientToken,
    });
    if (!shareAvailable) {
      throw new Error(
        `Share grant ${shareGrantId} not found on recipient node after 30s`,
      );
    }

    await clickNav(pageC, T.navSharedWithMe);

    // Wait for share row to appear in UI (should be quick since API confirmed it exists)
    let streamRowFound = false;
    for (let index = 0; index < 10; index++) {
      const row = pageC
        .getByTestId(`incoming-share-row-${collectionTitle}`)
        .first();
      if ((await row.count()) > 0) {
        streamRowFound = true;
        await row.getByTestId('incoming-share-open').click();
        break;
      }

      await pageC.waitForTimeout(500);
    }

    expect(streamRowFound).toBe(true);

    await expect(pageC.getByTestId('shared-manifest')).toBeVisible({
      timeout: 15_000,
    });

    const streamUrl = await incomingMovieStream({
      request, recipient: nodeC, recipientToken, owner: nodeA, title: collectionTitle,
    });

    const popupPromise = pageC.waitForEvent('popup');
    await pageC.getByTestId('incoming-stream-93df4e31').click();
    const playback = await popupPromise;
    await expect.poll(() => playback.evaluate(() => {
      const video = document.querySelector('video');
      return Boolean(video && video.readyState >= 2 && video.videoWidth > 0
        && video.getVideoPlaybackQuality().totalVideoFrames > 0 && !video.error);
    }), { timeout: 20_000 }).toBe(true);
    expect(new URL(playback.url()).searchParams.has('ticket')).toBe(true);
    expect(new URL(playback.url()).searchParams.has('token')).toBe(false);

    const normalized = streamUrl
      .replace('http://localhost:', 'http://127.0.0.1:')
      .replace('https://localhost:', 'https://127.0.0.1:');
    const fullStreamUrl = normalized.startsWith('http')
      ? normalized
      : `${nodeC.baseUrl}${normalized}`;

    const streamResponse = await request.get(fullStreamUrl, {
      failOnStatusCode: false,
      headers: { Range: 'bytes=0-1' },
    });
    const status = streamResponse.status();
    expect([200, 206]).toContain(status);

    const contentType = streamResponse.headers()['content-type'];
    if (status === 206) {
      // Streaming supports any shared content; keep this broad so E2E isn't coupled
      // to local share-indexing heuristics for large media files.
      expect(contentType).toMatch(/video|audio|application|text|image/i);
    }

    await contextA.close();
    await contextC.close();
  });

  test('recipient_backfills_and_verifies_download', async ({
    browser,
    request,
  }) => {
    const nodeCInstance = harness ? harness.getNode('C') : null;
    test.skip(
      !nodeCInstance,
      'Recipient backfill proof requires the locally launched pinned mesh peer',
    );
    const nodeC = nodeCInstance ? nodeCInstance.nodeCfg : NODES.C;
    await waitForHealth(request, nodeC.baseUrl);

    const contextC = await browser.newContext();
    const pageC = await contextC.newPage();
    await login(pageC, nodeC);

    const nodeA = harness!.getNode('A').nodeCfg;
    await waitForHealth(request, nodeA.baseUrl);
    const contextA = await browser.newContext();
    const pageA = await contextA.newPage();
    await login(pageA, nodeA);
    const ownerToken = await getAuthToken(pageA);
    const ownerHeaders = { Authorization: `Bearer ${ownerToken}` };
    const capabilityProbe = await request.post(
      `${nodeA.baseUrl}/api/overlay/connect`,
      {
        data: { username: nodeC.username },
        headers: ownerHeaders,
        failOnStatusCode: false,
      },
    );
    expect(capabilityProbe.status()).toBe(202);
    await expect
      .poll(
        async () => {
          const response = await request.get(
            `${nodeA.baseUrl}/api/soulseek/peer-capabilities`,
            { headers: ownerHeaders, failOnStatusCode: false },
          );
          if (!response.ok()) return false;
          const records = await response.json();
          return (
            Array.isArray(records) &&
            records.some((record) =>
              record?.username?.toLowerCase() === nodeC.username.toLowerCase(),
            )
          );
        },
        { timeout: 30_000, intervals: [250, 500, 1_000] },
      )
      .toBe(true);

    const backfillCollectionTitle = 'E2E Backfill Fixture';
    const recipientToken = await getAuthToken(pageC);
    const backfillCollectionResponse = await request.post(
      `${nodeA.baseUrl}/api/v0/collections`,
      {
        data: { title: backfillCollectionTitle, type: 'Playlist' },
        headers: ownerHeaders,
      },
    );
    expect(backfillCollectionResponse.status()).toBe(201);
    const backfillCollection = await backfillCollectionResponse.json();
    const backfillLibraryResponse = await request.get(
      `${nodeA.baseUrl}/api/v0/library/items?query=treasure_island_pg120.txt&limit=1`,
      { headers: ownerHeaders },
    );
    expect(backfillLibraryResponse.status()).toBe(200);
    const backfillLibraryPayload = await backfillLibraryResponse.json();
    const backfillItem = backfillLibraryPayload?.items?.[0];
    const backfillContentId =
      backfillItem?.contentId || backfillItem?.content_id;
    expect(backfillContentId).toBeTruthy();
    const addBackfillItemResponse = await request.post(
      `${nodeA.baseUrl}/api/v0/collections/${backfillCollection.id}/items`,
      {
        data: {
          contentId: backfillContentId,
          mediaKind: backfillItem?.mediaKind || backfillItem?.media_kind,
        },
        headers: ownerHeaders,
      },
    );
    expect(addBackfillItemResponse.ok()).toBe(true);
    const backfillGrantResponse = await request.post(
      `${nodeA.baseUrl}/api/v0/share-grants`,
      {
        data: {
          collectionId: backfillCollection.id,
          username: nodeC.username,
          allowDownload: true,
          allowStream: true,
          allowReshare: false,
        },
        headers: ownerHeaders,
      },
    );
    expect(backfillGrantResponse.status()).toBe(201);
    const backfillGrant = await backfillGrantResponse.json();
    expect(backfillGrant?.id).toBeTruthy();
    await announceShareGrant({
      owner: nodeA,
      ownerToken,
      recipient: nodeC,
      recipientToken,
      request,
      shareGrantId: backfillGrant.id,
    });
    await expect
      .poll(
        () =>
          waitForShareGrantById({
            baseUrl: nodeC.baseUrl,
            request,
            shareGrantId: backfillGrant.id,
            timeoutMs: 1_000,
            token: recipientToken,
          }),
        { timeout: 10_000, intervals: [250, 500, 1_000] },
      )
      .toBe(true);

    await clickNav(pageC, T.navSharedWithMe);

    let rowFound = false;
    for (let index = 0; index < 30; index += 1) {
      const row = pageC
        .getByTestId(`incoming-share-row-${backfillCollectionTitle}`)
        .first();
      if ((await row.count()) > 0) {
        rowFound = true;
        await row.getByTestId('incoming-share-open').click();
        break;
      }

      await pageC.waitForTimeout(500); // Reduced from 1000ms
    }

    expect(rowFound).toBe(true);

    await expect(pageC.getByTestId('shared-manifest')).toBeVisible({
      timeout: 15_000,
    });

    const backfillButton = pageC.getByTestId('incoming-backfill');
    await expect(backfillButton).toBeVisible();
    const backfillResponsePromise = pageC.waitForResponse(
      (response) =>
        response.url().includes('/api/v0/share-grants/') &&
        response.url().endsWith('/backfill') &&
        response.request().method() === 'POST',
      { timeout: 120_000 },
    );
    await backfillButton.click();
    const backfillResponse = await backfillResponsePromise;
    const backfillResponseBody = await backfillResponse.text();
    expect(backfillResponse.status(), backfillResponseBody).toBe(200);
    const backfillBody = JSON.parse(backfillResponseBody);
    expect(backfillBody.backfilled).toBe(1);
    expect(backfillBody.failed).toBe(0);
    expect(backfillBody.files).toEqual([
      expect.objectContaining({
        size: 399_906,
        sha256:
          '2e93caf3f954e8e8457d9846ad7756f74ccf192dab77b7247d48ba134a8e2c1b',
      }),
    ]);

    if (nodeCInstance) {
      const treasureFile = await nodeCInstance.waitForDownloadedFile(
        'sha256_2e93caf3f954e8e8457d9846ad7756f74ccf192dab77b7247d48ba134a8e2c1b',
        5_000,
        100,
      );
      expect(treasureFile).not.toBeNull();
      expect(treasureFile?.size).toBe(399_906);
      const content = await readFile(treasureFile!.path);
      expect(createHash('sha256').update(content).digest('hex')).toBe(
        '2e93caf3f954e8e8457d9846ad7756f74ccf192dab77b7247d48ba134a8e2c1b',
      );
    }

    await contextC.close();
    await contextA.close();
  });
});
