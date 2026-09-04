import api from './api';
import { getNetworkStats } from './runtimeCapabilities';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('runtime capability API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps a null compatibility result for an absent network endpoint', async () => {
    api.get.mockRejectedValue({ response: { status: 404 } });

    await expect(getNetworkStats()).resolves.toBeNull();
  });

  it('does not hide an unavailable daemon behind a null snapshot', async () => {
    const error = new Error('connection refused');
    api.get.mockRejectedValue(error);

    await expect(getNetworkStats()).rejects.toBe(error);
  });
});
