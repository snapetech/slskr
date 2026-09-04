import api from './api';
import { updateFilters } from './wishlist';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
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
});
