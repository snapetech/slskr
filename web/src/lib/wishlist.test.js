import api from './api';
import { getAll, getIgnoredResults, getSearches, updateFilters } from './wishlist';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    put: vi.fn(),
  },
}));

describe('wishlist bulk filter helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('updates selected wishlist filters through the live bulk route', async () => {
    api.put.mockResolvedValue({ data: { updatedCount: 2 } });

    await expect(updateFilters(['wish/1', 'wish-2'], 'flac -live')).resolves.toEqual({
      updatedCount: 2,
    });
    expect(api.put).toHaveBeenCalledWith('/wishlist/bulk-filter', {
      filter: 'flac -live',
      ids: ['wish/1', 'wish-2'],
    });
  });

  it('rejects malformed wishlist list responses', async () => {
    api.get
      .mockResolvedValueOnce({ data: {} })
      .mockResolvedValueOnce({ data: {} })
      .mockResolvedValueOnce({ data: {} });

    await expect(getAll()).rejects.toThrow(
      'Wishlist API returned an invalid wishlist response',
    );
    await expect(getSearches('wish-1')).rejects.toThrow(
      'Wishlist API returned an invalid wishlist searches response',
    );
    await expect(getIgnoredResults('wish-1')).rejects.toThrow(
      'Wishlist API returned an invalid ignored results response',
    );
  });
});
