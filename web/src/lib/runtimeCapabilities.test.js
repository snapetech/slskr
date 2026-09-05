import api from './api';
import {
  getActiveSwarmJobs,
  getCapabilities,
  getMeshPeers,
  getNetworkStats,
  getRuntimeStats,
} from './runtimeCapabilities';
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

  it('rejects malformed runtime envelopes instead of normalizing them to empty state', async () => {
    api.get.mockResolvedValueOnce({ data: [] });
    await expect(getCapabilities()).rejects.toThrow(
      'Runtime API returned an invalid capabilities response',
    );

    api.get.mockResolvedValueOnce({ data: {} });
    await expect(getMeshPeers()).rejects.toThrow(
      'Runtime API returned an invalid mesh peer list response',
    );

    api.get.mockResolvedValueOnce({ data: { jobs: {} } });
    await expect(getActiveSwarmJobs()).rejects.toThrow(
      'Runtime API returned an invalid swarm job list response',
    );

    api.get.mockResolvedValueOnce({ data: [] });
    await expect(getNetworkStats()).rejects.toThrow(
      'Runtime API returned an invalid network stats response',
    );
  });

  it('keeps the unavailable compatibility state distinct from a normalized snapshot', async () => {
    api.get.mockRejectedValue({ response: { status: 404 } });

    await expect(getRuntimeStats()).resolves.toBeNull();
  });
});
