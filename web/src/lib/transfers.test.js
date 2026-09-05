import api from './api';
import {
  getAll,
  getChanges,
  getFlat,
  getHistory,
} from './transfers';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('transfer API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns validated list, change, and history responses', async () => {
    api.get
      .mockResolvedValueOnce({ data: [{ id: 'download-1' }] })
      .mockResolvedValueOnce({ data: [{ id: 'upload-1' }] })
      .mockResolvedValueOnce({
        data: {
          counts: { download: 1, upload: 2 },
          cursor: 1700000000000,
          transfers: [{ id: 'change-1' }],
        },
      })
      .mockResolvedValueOnce({
        data: {
          asOf: 1700000000000,
          hasMore: true,
          nextOffset: 10,
          transfers: [{ id: 'history-1' }],
        },
      });

    await expect(
      getAll({ direction: 'download', includeCompleted: false }),
    ).resolves.toEqual([{ id: 'download-1' }]);
    await expect(
      getFlat({ direction: 'upload', includeRemoved: true }),
    ).resolves.toEqual([{ id: 'upload-1' }]);
    await expect(getChanges()).resolves.toEqual({
      counts: { download: 1, upload: 2 },
      cursor: 1700000000000,
      transfers: [{ id: 'change-1' }],
    });
    await expect(
      getHistory({ direction: 'download', offset: 5, limit: 5 }),
    ).resolves.toEqual({
      asOf: 1700000000000,
      hasMore: true,
      nextOffset: 10,
      transfers: [{ id: 'history-1' }],
    });
  });

  it('rejects malformed list responses', async () => {
    api.get.mockResolvedValue({ data: {} });

    await expect(getAll({ direction: 'download' })).rejects.toThrow(
      'Transfers API returned an invalid download transfers response',
    );
    await expect(getFlat()).rejects.toThrow(
      'Transfers API returned an invalid flat transfers response',
    );
  });

  it.each([
    ['transfer changes', []],
    ['transfer change counts', { cursor: 1, transfers: [] }],
    [
      'download transfer count',
      { counts: { upload: 0 }, cursor: 1, transfers: [] },
    ],
    [
      'transfer change cursor',
      { counts: { download: 0, upload: 0 }, cursor: 'unknown', transfers: [] },
    ],
    [
      'transfer changes',
      { counts: { download: 0, upload: 0 }, cursor: 1, transfers: {} },
    ],
  ])('rejects malformed %s responses', async (resource, data) => {
    api.get.mockResolvedValue({ data });

    await expect(getChanges()).rejects.toThrow(
      `Transfers API returned an invalid ${resource}`,
    );
  });

  it.each([
    ['transfer history', {}],
    [
      'transfer history timestamp',
      { hasMore: false, nextOffset: 0, transfers: [], asOf: 'unknown' },
    ],
    [
      'transfer history offset',
      { asOf: 1, hasMore: false, nextOffset: 'unknown', transfers: [] },
    ],
    [
      'transfer history hasMore',
      { asOf: 1, nextOffset: 0, transfers: [] },
    ],
    [
      'transfer history',
      { asOf: 1, hasMore: false, nextOffset: 0, transfers: {} },
    ],
  ])('rejects malformed %s responses', async (resource, data) => {
    api.get.mockResolvedValue({ data });

    await expect(getHistory({ direction: 'download' })).rejects.toThrow(
      `Transfers API returned an invalid ${resource}`,
    );
  });
});
