import { getForensicMatrix, getRuns } from './songid';
import api from './api';

vi.mock('./api', () => ({
  default: {
    get: vi.fn(),
  },
}));

describe('songId api helpers', () => {
  beforeEach(() => {
    api.get.mockReset();
  });

  it('loads the explicit forensic matrix export endpoint', async () => {
    api.get.mockResolvedValue({
      data: {
        identityScore: 91,
        syntheticScore: 12,
      },
    });

    const matrix = await getForensicMatrix('run/id');

    expect(api.get).toHaveBeenCalledWith('/songid/runs/run%2Fid/forensic-matrix');
    expect(matrix).toEqual({
      identityScore: 91,
      syntheticScore: 12,
    });
  });

  it('serializes the run limit as a query parameter', async () => {
    api.get.mockResolvedValue({ data: [] });

    await expect(getRuns(25)).resolves.toEqual([]);

    expect(api.get).toHaveBeenCalledWith('/songid/runs?limit=25');
  });

  it('rejects malformed run lists', async () => {
    api.get.mockResolvedValue({ data: {} });

    await expect(getRuns()).rejects.toThrow(
      'SongID API returned an invalid run list response',
    );
  });
});
