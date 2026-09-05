import api from './api';
import {
  getCapabilities,
  getActiveSwarmJobs,
  getMetadataProcessingStatus,
  getDiscoveredPeers,
  getHashDatabaseStats,
  getMeshPeers,
  getMeshStats,
  getSlskrStats,
} from './slskr';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('slskr runtime API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('uses compatibility defaults only when an optional endpoint is absent', async () => {
    api.get.mockRejectedValue({ response: { status: 404 } });

    await expect(getCapabilities()).resolves.toEqual({ features: [] });
  });

  it('surfaces server failures instead of presenting empty runtime data', async () => {
    const error = { response: { status: 503 } };
    api.get.mockRejectedValue(error);

    await expect(getMetadataProcessingStatus()).rejects.toBe(error);
  });

  it('propagates real failures from the combined runtime snapshot', async () => {
    const error = new Error('runtime unavailable');
    api.get.mockRejectedValue(error);

    await expect(getSlskrStats()).rejects.toBe(error);
  });

  it('normalizes peer endpoint envelopes for the network dashboard', async () => {
    api.get
      .mockResolvedValueOnce({
        data: {
          count: 1,
          peers: [{ username: 'mesh-peer' }, null, 'invalid-peer'],
        },
      })
      .mockResolvedValueOnce({
        data: [{ username: 'discovered-peer' }],
      });

    await expect(getMeshPeers()).resolves.toEqual([{ username: 'mesh-peer' }]);
    await expect(getDiscoveredPeers()).resolves.toEqual([
      { username: 'discovered-peer' },
    ]);
  });

  it('normalizes the multisource jobs envelope for runtime stats', async () => {
    api.get.mockResolvedValue({
      data: {
        count: 1,
        jobs: [{ jobId: 'swarm-1', status: 'in_progress' }, null, 'invalid-job'],
      },
    });

    await expect(getActiveSwarmJobs()).resolves.toEqual([
      {
        activeSources: 0,
        downloadedBytes: 0,
        jobId: 'swarm-1',
        progressPercent: 0,
        status: 'in_progress',
        totalBytes: 0,
      },
    ]);
  });

  it('rejects malformed runtime envelopes instead of manufacturing empty state', async () => {
    api.get.mockResolvedValueOnce({ data: [] });
    await expect(getCapabilities()).rejects.toThrow(
      'slskR API returned an invalid capabilities response',
    );

    api.get.mockResolvedValueOnce({ data: {} });
    await expect(getMeshPeers()).rejects.toThrow(
      'slskR API returned an invalid peer list response',
    );

    api.get.mockResolvedValueOnce({ data: { jobs: {} } });
    await expect(getActiveSwarmJobs()).rejects.toThrow(
      'slskR API returned an invalid swarm job list response',
    );

    api.get.mockResolvedValueOnce({ data: [] });
    await expect(getHashDatabaseStats()).rejects.toThrow(
      'slskR API returned an invalid hash database stats response',
    );

    api.get.mockResolvedValueOnce({ data: [] });
    await expect(getMeshStats()).rejects.toThrow(
      'slskR API returned an invalid mesh stats response',
    );
  });
});
