import { chromium } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';

const baseUrl = process.env.SLSKR_SCREENSHOT_BASE_URL || 'http://127.0.0.1:3001';
const outputDir =
  process.env.SLSKR_SCREENSHOT_OUTPUT_DIR ||
  path.resolve(process.cwd(), '../docs/screenshots');

const searches = [
  {
    endedAt: new Date('2026-05-05T18:20:00Z').toISOString(),
    fileCount: 18,
    id: 'commons-example-sound',
    lockedFileCount: 0,
    responseCount: 7,
    searchText: 'Example sound file Ogg Vorbis',
    startedAt: new Date('2026-05-05T18:18:00Z').toISOString(),
    state: 'Completed',
  },
  {
    endedAt: null,
    fileCount: 4,
    id: 'commons-click-track',
    lockedFileCount: 1,
    responseCount: 3,
    searchText: 'Audacity click track eight seconds',
    startedAt: new Date('2026-05-05T18:22:00Z').toISOString(),
    state: 'InProgress',
  },
];

const transfers = [
  {
    username: 'commons_peer',
    directories: [
      {
        directory: 'open-fixtures',
        files: [
          {
            averageSpeed: 96_000,
            bytesRemaining: 0,
            bytesTransferred: 153_301,
            direction: 'Download',
            elapsedTime: '00:00:02',
            filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg',
            id: 'transfer-commons-example-sound',
            percentComplete: 100,
            placeInQueue: 0,
            remainingTime: '00:00:00',
            size: 153_301,
            startOffset: 0,
            state: 'Completed, Succeeded',
            username: 'commons_peer',
          },
          {
            averageSpeed: 64_000,
            bytesRemaining: 3_820,
            bytesTransferred: 3_820,
            direction: 'Download',
            elapsedTime: '00:00:01',
            filename: 'Audacity_click_track_one_per_second_for_eight_seconds_mono88khz32bitfloat.ogg',
            id: 'transfer-commons-click-track',
            percentComplete: 50,
            placeInQueue: 1,
            remainingTime: '00:00:01',
            size: 7_640,
            startOffset: 0,
            state: 'InProgress',
            username: 'commons_peer',
          },
        ],
      },
    ],
  },
];

const rooms = ['ambient', 'field-recordings', 'netlabel'];

const conversations = [
  {
    id: 'conversation-audio-lab',
    messages: [
      {
        direction: 'Incoming',
        id: 'message-1',
        isAcknowledged: false,
        sentAt: new Date('2026-05-05T18:10:00Z').toISOString(),
        text: 'The click-track folder is browseable now.',
      },
    ],
    username: 'audio_lab',
  },
  {
    id: 'conversation-commons-peer',
    messages: [
      {
        direction: 'Outgoing',
        id: 'message-2',
        isAcknowledged: true,
        sentAt: new Date('2026-05-05T18:12:00Z').toISOString(),
        text: 'Thanks, queued the sample file.',
      },
    ],
    username: 'commons_peer',
  },
];

const applicationState = {
  connectionWatchdog: {
    lastCheckAt: new Date('2026-05-05T18:24:00Z').toISOString(),
    status: 'healthy',
  },
  relay: { mode: 'direct' },
  server: {
    address: 'vps.slsknet.org:2271',
    isConnected: true,
    isConnecting: false,
    username: 'local_operator',
  },
  shares: {
    directoryCount: 4,
    fileCount: 128,
    scanPending: false,
    scannedAt: new Date('2026-05-05T18:00:00Z').toISOString(),
    size: 3_400_000_000,
  },
  transfers: {
    down: 64_000,
    up: 12_000,
  },
  user: {
    username: 'local_operator',
  },
  version: {
    current: '0.0.0',
    isUpdateAvailable: false,
    latest: '0.0.0',
  },
  vpn: {
    isReady: false,
  },
};

const applicationOptions = {
  directories: {
    downloads: '/srv/slskr/downloads',
    incomplete: '/srv/slskr/incomplete',
  },
  shares: {
    directories: ['/srv/media/open-fixtures'],
  },
};

const json = (data) => ({
  body: JSON.stringify(data),
  contentType: 'application/json',
  status: 200,
});

const fallback = (url) => {
  const pathname = new URL(url).pathname.replace(/^\/api\/v0/, '').replace(/^\/api/, '');

  if (pathname === '/session/enabled') return json(true);
  if (pathname === '/session') return json({ username: 'local_operator' });
  if (pathname === '/application') return json(applicationState);
  if (pathname === '/options') return json(applicationOptions);
  if (pathname === '/server') return json(applicationState.server);
  if (pathname === '/application/version/latest') return json(applicationState.version);
  if (pathname === '/capabilities') {
    return json({
      api: ['health', 'events', 'metrics', 'telemetry'],
      feature: { scenePodBridge: false },
      network: ['server-session', 'peer-messaging', 'file-transfer'],
      storage: ['share-index', 'transfer-state'],
    });
  }
  if (pathname.startsWith('/searches/')) return json(searches[0]);
  if (pathname === '/searches') return json(searches);
  if (pathname === '/transfers/downloads') return json(transfers);
  if (pathname === '/transfers/uploads') return json([]);
  if (pathname === '/transfers/speeds') return json({ download: 64_000, upload: 12_000 });
  if (pathname === '/transfers/downloads/accelerated') return json({ enabled: true });
  if (pathname === '/transfers/downloads/user-stats') {
    return json({
      audio_lab: { failedDownloads: 0, successfulDownloads: 3 },
      commons_peer: { failedDownloads: 0, successfulDownloads: 12 },
    });
  }
  if (pathname === '/rooms/available') return json(rooms);
  if (pathname === '/rooms/joined') return json(['ambient', 'netlabel']);
  if (pathname.includes('/rooms/joined/') && pathname.endsWith('/messages')) {
    return json([
      {
        body: 'New open fixture set is mirrored.',
        direction: 'In',
        timestamp: new Date('2026-05-05T18:16:00Z').toISOString(),
        username: 'commons_peer',
      },
    ]);
  }
  if (pathname.includes('/rooms/joined/') && pathname.endsWith('/users')) {
    return json(['local_operator', 'commons_peer', 'audio_lab']);
  }
  if (pathname === '/conversations') return json(conversations);
  if (pathname.startsWith('/conversations/')) return json(conversations[0]);
  if (pathname === '/wishlist') {
    return json([
      {
        id: 'wishlist-1',
        searchText: 'Example sound file Ogg Vorbis',
        createdAt: new Date('2026-05-05T17:55:00Z').toISOString(),
      },
    ]);
  }
  if (pathname === '/users/notes') return json([]);
  if (pathname.startsWith('/users/notes/')) return json({});
  if (pathname.startsWith('/users/') && pathname.endsWith('/browse')) {
    return json({
      directories: [
        {
          files: [
            {
              filename: 'Example_sound_file_in_Ogg_Vorbis_format.ogg',
              size: 153_301,
            },
          ],
          name: 'open-fixtures',
        },
      ],
      username: 'commons_peer',
    });
  }
  if (pathname.startsWith('/users/')) return json({});
  if (pathname === '/shares') {
    return json({
      directories: applicationOptions.shares.directories,
      fileCount: 128,
      size: 3_400_000_000,
    });
  }
  if (pathname === '/shares/contents') {
    return json([
      {
        filename: 'commons-example-sound.ogg',
        size: 153_301,
      },
      {
        filename: 'commons-click-track.ogg',
        size: 7_640,
      },
    ]);
  }
  if (pathname === '/events') return json([]);
  if (pathname.includes('/metrics')) return json({});
  if (pathname.includes('/status')) return json({ enabled: false, status: 'disabled' });

  return json([]);
};

await fs.mkdir(outputDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });

await page.addInitScript(({ appState, options, searchList }) => {
  window.localStorage.setItem('slskr-theme', 'slskr');
  window.sessionStorage.setItem('slskr-token', 'readme-screenshot-token');

  class FakeWebSocket {
    constructor(url) {
      this.url = url;
      this.readyState = 0;
      setTimeout(() => {
        this.readyState = 1;
        this.onopen?.({});
        if (String(url).includes('/api/events/ws')) {
          this.onmessage?.({
            data: JSON.stringify({
              data: searchList,
              topic: 'search',
              type: 'search.list',
            }),
          });
          this.onmessage?.({
            data: JSON.stringify({
              data: appState,
              topic: 'application',
              type: 'session.updated',
            }),
          });
          this.onmessage?.({
            data: JSON.stringify({
              data: options,
              topic: 'application',
              type: 'config.updated',
            }),
          });
        }
      }, 50);
    }

    close() {
      this.readyState = 3;
      this.onclose?.({});
    }

    send() {}
  }

  FakeWebSocket.CONNECTING = 0;
  FakeWebSocket.OPEN = 1;
  FakeWebSocket.CLOSING = 2;
  FakeWebSocket.CLOSED = 3;
  window.WebSocket = FakeWebSocket;
}, { appState: applicationState, options: applicationOptions, searchList: searches });

await page.route('**/api/v0/**', (route) => route.fulfill(fallback(route.request().url())));
await page.route('**/api/**', (route) => route.fulfill(fallback(route.request().url())));

const capture = async (route, filename) => {
  await page.goto(`${baseUrl}${route}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(600);
  await page.screenshot({
    fullPage: false,
    path: path.join(outputDir, filename),
  });
};

await capture('/searches', 'webui-searches.png');
await capture('/downloads', 'webui-downloads.png');
await capture('/rooms', 'webui-rooms.png');
await capture('/system', 'webui-system.png');

await browser.close();
