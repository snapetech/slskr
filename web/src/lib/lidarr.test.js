import api from './api';
import { getImportHistory, getWantedMissing, retryImport } from './lidarr';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe('lidarr manual import history helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads bounded import history through the versioned relative route', async () => {
    api.get.mockResolvedValue({ data: [] });

    await getImportHistory({ limit: 25 });

    expect(api.get).toHaveBeenCalledWith(
      '/integrations/lidarr/manualimport/history?limit=25',
    );
  });

  it('encodes wanted pagination parameters', async () => {
    api.get.mockResolvedValue({ data: [] });

    await getWantedMissing({ page: '1&bad', pageSize: '25?bad' });

    expect(api.get).toHaveBeenCalledWith(
      '/integrations/lidarr/wanted/missing?page=1%26bad&pageSize=25%3Fbad',
    );
  });

  it('encodes history IDs before retrying an import', async () => {
    api.post.mockResolvedValue({ data: { id: 'retry-1' } });

    await retryImport('history/with spaces');

    expect(api.post).toHaveBeenCalledWith(
      '/integrations/lidarr/manualimport/history/history%2Fwith%20spaces/retry',
    );
  });
});
