import api from './api';
import { browse, browseAll, get, getAll } from './shares';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('shares API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns share and contents responses with their expected shapes', async () => {
    api.get
      .mockResolvedValueOnce({ data: { local: [] } })
      .mockResolvedValueOnce({ data: { id: 'share-1' } })
      .mockResolvedValueOnce({ data: [{ name: 'Music', files: [] }] })
      .mockResolvedValueOnce({ data: [{ name: 'Albums', files: [] }] });

    await expect(getAll()).resolves.toEqual({ local: [] });
    await expect(get({ id: 'share-1' })).resolves.toEqual({ id: 'share-1' });
    await expect(browseAll()).resolves.toEqual([
      { name: 'Music', files: [] },
    ]);
    await expect(browse({ id: 'share-1' })).resolves.toEqual([
      { name: 'Albums', files: [] },
    ]);
  });

  it.each([
    ['share list', getAll],
    ['share', () => get({ id: 'share-1' })],
    ['share contents', browseAll],
    ['share contents', () => browse({ id: 'share-1' })],
  ])('rejects malformed %s responses', async (resource, request) => {
    api.get.mockResolvedValue({ data: resource.includes('contents') ? {} : [] });

    await expect(request()).rejects.toThrow(
      `Shares API returned an invalid ${resource} response`,
    );
  });
});
