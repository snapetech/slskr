import api from './api';
import { getLogs } from './options';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('options API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns log envelopes unchanged', async () => {
    const envelope = {
      entries: [{ context: 'daemon', level: 'Information', message: 'ready' }],
      level: 'Debug',
      levels: ['Debug', 'Information'],
      limit: 500,
    };
    api.get.mockResolvedValueOnce({ data: envelope });

    await expect(getLogs()).resolves.toEqual(envelope);
    expect(api.get).toHaveBeenCalledTimes(1);
    expect(api.get).toHaveBeenCalledWith('/logs');
  });

  it('wraps slskd-compatible log arrays for the web logs view', async () => {
    const entries = [
      {
        context: 'session',
        level: 'Information',
        message: 'connect requested',
        timestamp: 1_783_465_916,
      },
    ];
    api.get
      .mockResolvedValueOnce({ data: entries })
      .mockResolvedValueOnce({
        data: {
          level: 'Warning',
          levels: ['Information', 'Warning', 'Error'],
        },
      });

    await expect(getLogs()).resolves.toEqual({
      entries,
      level: 'Warning',
      levels: ['Information', 'Warning', 'Error'],
      limit: 1,
    });
    expect(api.get).toHaveBeenNthCalledWith(1, '/logs');
    expect(api.get).toHaveBeenNthCalledWith(2, '/logs/level');
  });
});
