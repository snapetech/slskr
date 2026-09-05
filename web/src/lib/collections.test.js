import api from './api';
import {
  buildRemoteShareStreamUrl,
  createShareToken,
  fetchRemoteShareManifest,
  getShareManifest,
  remoteBackfillShare,
} from './collections';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe('share API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('keeps reusable share tokens out of request URLs', () => {
    getShareManifest('grant/1', 'secret/token');

    expect(api.get).toHaveBeenCalledWith('/share-grants/grant%2F1/manifest', {
      headers: { 'X-Share-Token': 'secret/token' },
    });
    expect(api.get.mock.calls[0][0]).not.toContain('secret/token');
  });

  it('encodes grant ids when creating tokens', () => {
    createShareToken('grant/1', 120);

    expect(api.post).toHaveBeenCalledWith('/share-grants/grant%2F1/token', {
      expiresInSeconds: 120,
    });
  });

  it('sends remote share tokens and refuses redirects', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{"items":[]}', {
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    await expect(
      fetchRemoteShareManifest('https://owner.example/base/', 'grant/1', 'share-secret'),
    ).resolves.toEqual({ items: [] });

    expect(fetchMock).toHaveBeenCalledWith(
      'https://owner.example/base/api/v0/share-grants/grant%2F1/manifest',
      expect.objectContaining({
        redirect: 'error',
        headers: { 'X-Share-Token': 'share-secret' },
      }),
    );
  });

  it('authenticates remote backfill with the delegated share token', async () => {
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('{"backfilled":1}', {
        status: 202,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    await expect(
      remoteBackfillShare('https://owner.example', 'grant/1', 'share-secret'),
    ).resolves.toEqual({ backfilled: 1 });

    expect(fetchMock.mock.calls[0][1]).toEqual(
      expect.objectContaining({
        method: 'POST',
        redirect: 'error',
        headers: {
          'X-Share-Token': 'share-secret',
          'Content-Type': 'application/json',
        },
      }),
    );
  });

  it('returns a useful error for non-JSON remote failures', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response('<html>bad gateway</html>', { status: 502 }),
    );

    await expect(
      fetchRemoteShareManifest('https://owner.example', 'grant-1', 'share-secret'),
    ).rejects.toMatchObject({ message: 'Request failed: 502' });
  });

  it('normalizes remote owner endpoints before building stream URLs', () => {
    expect(
      buildRemoteShareStreamUrl(
        'https://owner.example/base/?ignored=query',
        'content/1',
        'ticket/1',
      ),
    ).toBe('https://owner.example/base/api/v0/streams/content%2F1?ticket=ticket%2F1');
  });
});
