import api from './api';
import {
  disableAutoReplace,
  enableAutoReplace,
  findAlternative,
  getAutoReplaceStatus,
  getStuckDownloads,
  processStuckDownloads,
  replaceDownload,
} from './autoReplace';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
}));

describe('auto-replace API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns validated list, status, and mutation responses', async () => {
    api.get
      .mockResolvedValueOnce({ data: [{ id: 1 }] })
      .mockResolvedValueOnce({ data: { enabled: false } });
    api.post
      .mockResolvedValueOnce({ data: [{ username: 'peer' }] })
      .mockResolvedValueOnce({ data: { queued: true } })
      .mockResolvedValueOnce({ data: { processed: 1 } });
    api.put
      .mockResolvedValueOnce({ data: { enabled: true } })
      .mockResolvedValueOnce({ data: { enabled: false } });

    await expect(getStuckDownloads()).resolves.toEqual([{ id: 1 }]);
    await expect(getAutoReplaceStatus()).resolves.toEqual({ enabled: false });
    await expect(
      findAlternative({ filename: 'song.flac', size: 42, username: 'peer' }),
    ).resolves.toEqual([{ username: 'peer' }]);
    await expect(
      replaceDownload({
        newFilename: 'new.flac',
        newSize: 42,
        newUsername: 'other',
        originalId: '1',
        originalUsername: 'peer',
      }),
    ).resolves.toEqual({ queued: true });
    await expect(processStuckDownloads({ threshold: 5 })).resolves.toEqual({
      processed: 1,
    });
    await expect(enableAutoReplace()).resolves.toEqual({ enabled: true });
    await expect(disableAutoReplace()).resolves.toEqual({ enabled: false });
  });

  it.each([
    ['stuck downloads', getStuckDownloads, 'get', {}],
    ['status', getAutoReplaceStatus, 'get', {}],
    ['replacement', replaceDownload, 'post', []],
    ['process', processStuckDownloads, 'post', []],
    ['enable', enableAutoReplace, 'put', []],
    ['disable', disableAutoReplace, 'put', []],
  ])('rejects malformed %s responses', async (_, helper, method, data) => {
    api[method].mockResolvedValue({ data });
    const promise =
      helper === replaceDownload
        ? helper({
            newFilename: 'new.flac',
            newSize: 42,
            newUsername: 'other',
            originalId: '1',
            originalUsername: 'peer',
          })
        : helper === processStuckDownloads
          ? helper({ threshold: 5 })
          : helper();
    await expect(promise).rejects.toThrow('Auto-replace API returned an invalid');
  });
});
