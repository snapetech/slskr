#!/usr/bin/env node

const args = process.argv.slice(2);
function option(name) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

const referenceBase = option('--reference-base');
const candidateBase = option('--candidate-base');
if (!referenceBase || !candidateBase) {
  process.stderr.write(
    'usage: check-slskdn-safe-mutation-contracts.mjs --reference-base URL --candidate-base URL\n',
  );
  process.exit(2);
}

const tokens = {
  reference: option('--reference-token'),
  candidate: option('--candidate-token'),
};
const failures = [];
let checks = 0;
let knownExceptions = 0;

async function request(side, method, route, body) {
  const base = side === 'reference' ? referenceBase : candidateBase;
  const headers = { Accept: 'application/json' };
  if (body !== undefined) headers['Content-Type'] = 'application/json';
  if (tokens[side]) headers.Authorization = `Bearer ${tokens[side]}`;
  const response = await fetch(`${base.replace(/\/$/, '')}${route}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let json = null;
  if (text) {
    try {
      json = JSON.parse(text);
    } catch {}
  }
  return { status: response.status, text, json };
}

function check(condition, message) {
  checks += 1;
  if (!condition) failures.push(message);
}

function objectKeys(value, required, label) {
  check(value && typeof value === 'object' && !Array.isArray(value), `${label}: expected object`);
  for (const key of required) check(Object.hasOwn(value ?? {}, key), `${label}: missing ${key}`);
}

const unique = Date.now().toString(16).slice(-8).padStart(8, '0');
const collections = {};
for (const side of ['reference', 'candidate']) {
  const response = await request(side, 'POST', '/api/v0/collections', {
    title: `Route Audit ${unique}`,
    description: 'safe contract probe',
    type: 'Playlist',
  });
  check(response.status === 201, `${side} collection: expected 201, got ${response.status}`);
  objectKeys(
    response.json,
    ['id', 'ownerUserId', 'title', 'description', 'type', 'createdAt', 'updatedAt'],
    `${side} collection`,
  );
  check(response.json?.type === 'Playlist', `${side} collection: type is not Playlist`);
  collections[side] = response.json?.id;
}

for (const side of ['reference', 'candidate']) {
  if (!collections[side]) continue;
  const response = await request(
    side,
    'POST',
    `/api/v0/collections/${encodeURIComponent(collections[side])}/items`,
    {
      contentId: `content:music:recording:${unique}`,
      mediaKind: 'Music',
      fileName: 'Route Audit.flac',
      title: 'Route Audit',
      artist: 'Contract Probe',
      album: 'Parity',
      contentHash: unique,
    },
  );
  check(response.status === 201, `${side} collection item: expected 201, got ${response.status}`);
  objectKeys(
    response.json,
    ['id', 'collectionId', 'ordinal', 'contentId', 'mediaKind', 'fileName', 'title', 'artist', 'album', 'contentHash'],
    `${side} collection item`,
  );
  check(response.json?.collectionId === collections[side], `${side} collection item: wrong collectionId`);
  const updatedCollection = await request(
    side,
    'PUT',
    `/api/v0/collections/${encodeURIComponent(collections[side])}`,
    { title: `Updated Route Audit ${unique}`, type: 'ShareList' },
  );
  check(updatedCollection.status === 200, `${side} collection update: expected 200`);
  check(updatedCollection.json?.description === 'safe contract probe', `${side} collection update: lost description`);
  check(updatedCollection.json?.type === 'ShareList', `${side} collection update: wrong type`);

  if (response.json?.id) {
    const updatedItem = await request(
      side,
      'PUT',
      `/api/v0/collections/${encodeURIComponent(collections[side])}/items/${encodeURIComponent(response.json.id)}`,
      { title: 'Updated Route Audit', album: 'Updated Parity', sha256: 'b'.repeat(64) },
    );
    check(updatedItem.status === 200, `${side} collection item update: expected 200`);
    check(updatedItem.json?.ordinal === 0, `${side} collection item update: wrong ordinal`);
    check(updatedItem.json?.contentHash === 'b'.repeat(64), `${side} collection item update: wrong hash`);
    const reordered = await request(
      side,
      'POST',
      `/api/v0/collections/${encodeURIComponent(collections[side])}/items/reorder`,
      { itemIds: [response.json.id] },
    );
    check(reordered.status === 204 && reordered.text === '', `${side} collection reorder: expected empty 204`);
    const deletedItem = await request(
      side,
      'DELETE',
      `/api/v0/collections/${encodeURIComponent(collections[side])}/items/${encodeURIComponent(response.json.id)}`,
    );
    check(deletedItem.status === 204, `${side} collection item cleanup: expected 204`);
  }
  const deleted = await request(
    side,
    'DELETE',
    `/api/v0/collections/${encodeURIComponent(collections[side])}`,
  );
  check(deleted.status === 204, `${side} collection cleanup: expected 204`);
}

for (const side of ['reference', 'candidate']) {
  const externalId = `route-audit-${unique}`;
  const contentId = `content:music:recording:${unique}`;
  const registered = await request(side, 'POST', '/api/v0/mediacore/contentid/register', {
    externalId,
    contentId,
  });
  check(registered.status === 200, `${side} content mapping: expected 200, got ${registered.status}`);
  check(
    registered.json?.message === 'ContentID mapping registered successfully',
    `${side} content mapping: unexpected response`,
  );
  const exists = await request(side, 'GET', `/api/v0/mediacore/contentid/exists/${externalId}`);
  check(exists.json?.exists === true, `${side} content mapping: exists did not observe write`);
  const external = await request(
    side,
    'GET',
    `/api/v0/mediacore/contentid/external/${encodeURIComponent(contentId)}`,
  );
  check(external.json?.externalIds?.includes(externalId), `${side} content mapping: reverse lookup failed`);
}

for (const side of ['reference', 'candidate']) {
  const wishlist = await request(side, 'POST', '/api/v0/wishlist', {
    searchText: `route-audit-${unique}`,
    filter: 'flac',
    enabled: true,
    autoDownload: false,
    maxResults: 10,
    maxDownloads: 1,
  });
  check(wishlist.status === 201, `${side} wishlist: expected 201, got ${wishlist.status}`);
  objectKeys(
    wishlist.json,
    ['id', 'searchText', 'filter', 'enabled', 'autoDownload', 'maxResults', 'createdAt'],
    `${side} wishlist`,
  );
  if (wishlist.json?.id) {
    const wishlistId = wishlist.json.id;
    const updated = await request(
      side,
      'PUT',
      `/api/v0/wishlist/${encodeURIComponent(wishlistId)}`,
      {
        searchText: `updated-route-audit-${unique}`,
        filter: 'lossless',
        enabled: true,
        autoDownload: false,
        maxResults: 5,
        maxDownloads: 1,
      },
    );
    check(updated.status === 200, `${side} wishlist update: expected 200`);
    check(updated.json?.searchText === `updated-route-audit-${unique}`, `${side} wishlist update: wrong searchText`);

    const ignored = await request(
      side,
      'POST',
      `/api/v0/wishlist/${encodeURIComponent(wishlistId)}/ignored-results`,
      { username: 'route-audit-peer', directory: '/tmp/slskdn-route-audit' },
    );
    check(ignored.status === 201, `${side} ignored result: expected 201`);
    objectKeys(
      ignored.json,
      ['id', 'wishlistItemId', 'username', 'directory', 'createdAt'],
      `${side} ignored result`,
    );
    check(ignored.json?.directory === '/tmp/slskdn-route-audit', `${side} ignored result: directory changed`);
    if (ignored.json?.id) {
      const removedIgnored = await request(
        side,
        'DELETE',
        `/api/v0/wishlist/${encodeURIComponent(wishlistId)}/ignored-results/${encodeURIComponent(ignored.json.id)}`,
      );
      check(removedIgnored.status === 204, `${side} ignored-result cleanup: expected 204`);
    }

    const viewed = await request(
      side,
      'POST',
      `/api/v0/wishlist/${encodeURIComponent(wishlistId)}/mark-viewed`,
    );
    check(viewed.status === 204, `${side} mark viewed: expected 204`);
    const markAll = await request(side, 'POST', '/api/v0/wishlist/mark-all-viewed');
    check(markAll.status === 204, `${side} mark all viewed: expected 204`);
    const deleted = await request(
      side,
      'DELETE',
      `/api/v0/wishlist/${encodeURIComponent(wishlistId)}`,
    );
    check(deleted.status === 204, `${side} wishlist cleanup: expected 204`);
  }

  const nowPlaying = await request(side, 'PUT', '/api/v0/nowplaying', {
    artist: 'Contract Probe',
    title: `Route Audit ${unique}`,
  });
  check(nowPlaying.status === 204 && nowPlaying.text === '', `${side} nowplaying: expected empty 204`);

  const quarantine = await request(side, 'POST', '/api/v0/quarantine-jury/requests', {
    evidence: [],
    jurors: [],
    minJurorVotes: 1,
  });
  check(quarantine.status === 400, `${side} quarantine: expected 400, got ${quarantine.status}`);
  objectKeys(quarantine.json, ['isValid', 'errors'], `${side} quarantine`);

  const username = `route-audit-${unique}`;
  const overlayBlock = await request(side, 'POST', '/api/v0/overlay/blocklist/username', {
    username,
    reason: 'safe contract probe',
  });
  check(overlayBlock.status === 200, `${side} overlay block: expected 200`);
  check(
    overlayBlock.json?.message === 'Username blocked',
    `${side} overlay block: unexpected response`,
  );
  await request(
    side,
    'DELETE',
    `/api/v0/overlay/blocklist/username/${encodeURIComponent(username)}`,
  );

  const securityBan = await request(side, 'POST', '/api/v0/security/bans/username', {
    username,
    reason: 'safe contract probe',
    durationMinutes: 1,
  });
  check(securityBan.status === 200 && securityBan.text === '', `${side} security ban: expected empty 200`);
  const securityBans = await request(side, 'GET', '/api/v0/security/bans');
  const securityRecord = securityBans.json?.find?.(
    (record) => record.key === `User:${username.toLowerCase()}`,
  );
  objectKeys(
    securityRecord,
    ['key', 'reason', 'bannedAt', 'expiresAt', 'isPermanent', 'timeRemaining'],
    `${side} security ban record`,
  );
  const securityUnban = await request(
    side,
    'DELETE',
    `/api/v0/security/bans/username/${encodeURIComponent(username)}`,
  );
  check(
    securityUnban.status === 200 && securityUnban.text === '',
    `${side} security unban: expected empty 200`,
  );
}

const emptyMetadataPackage = {
  version: '1.0',
  exportedAt: '2026-01-01T00:00:00Z',
  source: 'safe-contract-probe',
  entries: [],
  links: [],
  metadata: { totalEntries: 0, totalLinks: 0, entriesByDomain: {}, checksum: '' },
};
for (const side of ['reference', 'candidate']) {
  const fuzzyFind = await request(
    side,
    'POST',
    `/api/v0/mediacore/fuzzymatch/find/${encodeURIComponent(`content:music:recording:${unique}`)}`,
    {},
  );
  check(fuzzyFind.status === 200, `${side} fuzzy find: expected 200`);
  objectKeys(
    fuzzyFind.json,
    ['targetContentId', 'totalCandidates', 'matches', 'searchParameters'],
    `${side} fuzzy find`,
  );
  objectKeys(
    fuzzyFind.json?.searchParameters,
    ['minConfidence', 'maxCandidates', 'maxResults'],
    `${side} fuzzy find parameters`,
  );

  const fuzzyPerceptual = await request(side, 'POST', '/api/v0/mediacore/fuzzymatch/perceptual', {
    contentIdA: `content:music:recording:${unique}`,
    contentIdB: `content:music:recording:other-${unique}`,
  });
  check(fuzzyPerceptual.status === 200, `${side} fuzzy perceptual: expected 200`);
  objectKeys(
    fuzzyPerceptual.json,
    ['contentIdA', 'contentIdB', 'similarity', 'isSimilar', 'threshold'],
    `${side} fuzzy perceptual`,
  );

  const fuzzyText = await request(side, 'POST', '/api/v0/mediacore/fuzzymatch/text', {
    textA: 'same',
    textB: 'same',
  });
  check(fuzzyText.status === 200, `${side} fuzzy text: expected 200`);
  objectKeys(
    fuzzyText.json,
    ['textA', 'textB', 'levenshteinSimilarity', 'phoneticSimilarity', 'combinedSimilarity'],
    `${side} fuzzy text`,
  );
  check(fuzzyText.json?.combinedSimilarity === 1, `${side} fuzzy text: identical text did not score 1`);

  const emptyLinks = await request(
    side,
    'POST',
    `/api/v0/mediacore/ipld/links/${encodeURIComponent(`content:music:recording:${unique}`)}`,
    { links: [] },
  );
  check(emptyLinks.status === 400, `${side} empty IPLD links: expected 400`);
  const links = await request(
    side,
    'POST',
    `/api/v0/mediacore/ipld/links/${encodeURIComponent(`content:music:recording:${unique}`)}`,
    { links: [{ name: 'related', target: `content:music:recording:other-${unique}` }] },
  );
  check(links.status === 200, `${side} IPLD links: expected 200`);
  check(links.json?.message === 'Links added successfully', `${side} IPLD links: wrong result`);

  for (const [route, body] of [
    ['/api/v0/mediacore/perceptualhash/audio', { samples: [0.5], sampleRate: 1, algorithm: 'PHash' }],
    ['/api/v0/mediacore/perceptualhash/image', { pixels: 'AAAAAA==', width: 1, height: 1, algorithm: 'PHash' }],
  ]) {
    const hash = await request(side, 'POST', route, body);
    check(hash.status === 200, `${side} ${route}: expected 200`);
    objectKeys(hash.json, ['algorithm', 'hex', 'numericHash'], `${side} ${route}`);
    check(hash.json?.hex === '0000000000000000', `${side} ${route}: unexpected fixture hash`);
  }
  const similarity = await request(side, 'POST', '/api/v0/mediacore/perceptualhash/similarity', {
    hashA: '0000000000000000',
    hashB: 'ffffffffffffffff',
    threshold: 0.8,
  });
  objectKeys(
    similarity.json,
    ['hashA', 'hashB', 'hammingDistance', 'similarity', 'areSimilar', 'threshold'],
    `${side} perceptual similarity`,
  );
  check(similarity.json?.hammingDistance === 64, `${side} perceptual similarity: wrong distance`);

  const analyzed = await request(side, 'POST', '/api/v0/mediacore/portability/analyze', {
    package: emptyMetadataPackage,
  });
  objectKeys(
    analyzed.json,
    ['totalEntries', 'conflictingEntries', 'cleanEntries', 'conflicts', 'recommendedStrategies'],
    `${side} portability analyze`,
  );
  const emptyExport = await request(side, 'POST', '/api/v0/mediacore/portability/export', {
    contentIds: [],
  });
  check(emptyExport.status === 400, `${side} empty portability export: expected 400`);
  const imported = await request(side, 'POST', '/api/v0/mediacore/portability/import', {
    package: emptyMetadataPackage,
  });
  objectKeys(
    imported.json,
    ['success', 'entriesProcessed', 'entriesImported', 'entriesSkipped', 'conflictsResolved', 'conflicts', 'errors', 'duration'],
    `${side} portability import`,
  );

  const invalidPublish = await request(side, 'POST', '/api/v0/mediacore/publish/descriptor', {
    descriptor: { contentId: `content:music:recording:${unique}` },
  });
  check(invalidPublish.status === 400, `${side} unsigned descriptor publish: expected 400`);
  const emptyRepublish = await request(side, 'POST', '/api/v0/mediacore/publish/republish', {
    contentIds: [],
  });
  check(emptyRepublish.status === 400, `${side} empty republish: expected 400`);
  const missingUpdate = await request(
    side,
    'PUT',
    `/api/v0/mediacore/publish/descriptor/${encodeURIComponent(`content:music:recording:missing-${unique}`)}`,
    { updates: { notes: 'safe contract probe' } },
  );
  check(missingUpdate.status === 400, `${side} missing descriptor update: expected 400`);

  const verified = await request(side, 'POST', '/api/v0/mediacore/retrieve/verify', {
    descriptor: { contentId: `content:music:recording:${unique}`, hashes: [] },
  });
  objectKeys(
    verified.json,
    ['isValid', 'signatureValid', 'freshnessValid', 'age', 'validationError'],
    `${side} descriptor verification`,
  );
  check(verified.json?.isValid === false, `${side} descriptor verification: expected invalid`);
  const cleared = await request(side, 'POST', '/api/v0/mediacore/retrieve/cache/clear');
  objectKeys(cleared.json, ['success', 'entriesCleared', 'bytesFreed'], `${side} cache clear`);
  const reset = await request(side, 'POST', '/api/v0/mediacore/stats/reset');
  check(
    reset.json?.message === 'Statistics reset successfully',
    `${side} stats reset: wrong result`,
  );
}

const podBody = {
  podId: `pod:${unique.padStart(32, '0')}`,
  name: `Route Audit ${unique}`,
  visibility: 'Listed',
  contentId: `content:music:recording:${unique}`,
  tags: [],
  channels: [],
  externalBindings: [],
};
const referencePod = await request('reference', 'POST', '/api/v0/podcore/content/create-pod', podBody);
const candidatePod = await request('candidate', 'POST', '/api/v0/podcore/content/create-pod', podBody);
if (referencePod.status === 500 && candidatePod.status === 201) {
  knownExceptions += 1;
} else {
  check(
    referencePod.status === candidatePod.status,
    `pod creation: reference ${referencePod.status}, candidate ${candidatePod.status}`,
  );
}
await Promise.all(
  ['reference', 'candidate'].map((side) =>
    request(side, 'DELETE', `/api/v0/pods/${encodeURIComponent(podBody.podId)}`),
  ),
);

process.stderr.write(
  `slskdN safe mutation contracts: ${checks - failures.length}/${checks} checks passed; ` +
    `${knownExceptions} known upstream exceptions\n`,
);
if (failures.length) {
  for (const failure of failures) process.stderr.write(`- ${failure}\n`);
  process.exit(1);
}
