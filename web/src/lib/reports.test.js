import api from './api';
import {
  getExceptionPareto,
  getExceptions,
  getHistogram,
  getLeaderboard,
  getSummary,
  getTopDirectories,
} from './reports';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('transfer report helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.get.mockResolvedValue({ data: {} });
  });

  it('translates a requested bucket count into the daemon interval contract', async () => {
    const start = new Date('2026-01-01T00:00:00.000Z');
    const end = new Date('2026-01-02T00:00:00.000Z');

    await getHistogram({ buckets: 24, end, start });

    const request = api.get.mock.calls[0][0];
    const parameters = new URLSearchParams(request.split('?')[1]);
    expect(parameters.get('interval')).toBe('60');
    expect(parameters.get('buckets')).toBeNull();
  });

  it('honors an explicit interval while enforcing the daemon minimum', async () => {
    await getHistogram({ end: new Date(), interval: 2, start: new Date(0) });

    const request = api.get.mock.calls[0][0];
    const parameters = new URLSearchParams(request.split('?')[1]);
    expect(parameters.get('interval')).toBe('5');
  });

  it.each([
    ['summary', getSummary, {}],
    ['histogram', getHistogram, {}],
    ['leaderboard', getLeaderboard, []],
    ['directories', getTopDirectories, []],
    ['exceptions', getExceptions, []],
    ['exception pareto', getExceptionPareto, []],
  ])('returns a validated %s response', async (_, helper, data) => {
    api.get.mockResolvedValue({ data });
    await expect(helper()).resolves.toEqual(data);
  });

  it.each([
    ['summary', getSummary],
    ['histogram', getHistogram],
    ['leaderboard', getLeaderboard],
    ['directories', getTopDirectories],
    ['exceptions', getExceptions],
    ['exception pareto', getExceptionPareto],
  ])('rejects malformed %s responses', async (_, helper) => {
    api.get.mockResolvedValue({ data: 'malformed' });
    await expect(helper()).rejects.toThrow(
      'Transfer reports API returned an invalid',
    );
  });
});
