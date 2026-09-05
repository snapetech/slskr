import api from './api';
import { buildDiscoveryGraph } from './discoveryGraph';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    post: vi.fn(),
  },
}));

describe('discovery graph API helper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns a graph envelope', async () => {
    api.post.mockResolvedValue({ data: { edges: [], nodes: [] } });

    await expect(buildDiscoveryGraph({ artist: 'Example' })).resolves.toEqual({
      edges: [],
      nodes: [],
    });
  });

  it('rejects malformed graph envelopes', async () => {
    api.post.mockResolvedValue({ data: [] });

    await expect(buildDiscoveryGraph({ artist: 'Example' })).rejects.toThrow(
      'Discovery graph API returned an invalid graph response',
    );
  });
});
