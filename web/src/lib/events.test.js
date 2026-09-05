import api from './api';
import { list } from './events';

vi.mock('./api', () => ({
  __esModule: true,
  default: { get: vi.fn() },
}));

describe('event history API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('encodes optional filters while preserving pagination metadata', async () => {
    api.get.mockResolvedValue({
      data: [{ id: '1' }],
      headers: { 'x-total-count': '12' },
    });

    await expect(list({
      kind: 'search.started',
      limit: 10,
      offset: 20,
      q: 'ambient & live',
      topic: 'searches',
    })).resolves.toEqual({
      events: [{ id: '1' }],
      totalCount: '12',
    });

    expect(api.get).toHaveBeenCalledWith(
      '/events?offset=20&limit=10&kind=search.started&topic=searches&q=ambient+%26+live',
    );
  });

  it('omits blank filters', async () => {
    api.get.mockResolvedValue({ data: [], headers: {} });

    await list({ kind: '  ', limit: 10, offset: 0, q: '', topic: '\n' });

    expect(api.get).toHaveBeenCalledWith('/events?offset=0&limit=10');
  });

  it('rejects malformed event history responses', async () => {
    api.get.mockResolvedValue({ data: {} });

    await expect(list()).rejects.toThrow(
      'Events API returned an invalid event history response',
    );
  });
});
