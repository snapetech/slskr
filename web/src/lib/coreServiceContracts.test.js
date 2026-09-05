import api from './api';
import * as application from './application';
import * as destinations from './destinations';
import * as nowPlaying from './nowPlaying';
import * as server from './server';
import * as telemetry from './telemetry';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    delete: vi.fn(),
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
}));

describe('core service response contracts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns the documented object, list, text, and no-content shapes', async () => {
    api.get
      .mockResolvedValueOnce({ data: { runtimeProfile: 'native' } })
      .mockResolvedValueOnce({ data: { current: '1.0.0' } })
      .mockResolvedValueOnce({ data: { version: '1.0.0' } })
      .mockResolvedValueOnce({ data: { connected: true } })
      .mockResolvedValueOnce({ data: [{ path: '/downloads' }] })
      .mockResolvedValueOnce({ data: { path: '/downloads' } })
      .mockResolvedValueOnce({ data: { now_playing: [], count: 0 } })
      .mockResolvedValueOnce({ data: '# HELP slskr_metric gauge\n' })
      .mockResolvedValueOnce({ data: { slskr_metric: { samples: [] } } });
    api.post.mockResolvedValueOnce({ data: { valid: true } });

    await expect(application.getState()).resolves.toEqual({ runtimeProfile: 'native' });
    await expect(application.getVersion()).resolves.toEqual({ current: '1.0.0' });
    await expect(application.getBuild()).resolves.toEqual({ version: '1.0.0' });
    await expect(server.getState()).resolves.toEqual({ connected: true });
    await expect(destinations.getAll()).resolves.toEqual([{ path: '/downloads' }]);
    await expect(destinations.getDefault()).resolves.toEqual({ path: '/downloads' });
    await expect(destinations.validate('/downloads')).resolves.toEqual({ valid: true });
    await expect(nowPlaying.getNowPlaying()).resolves.toEqual({
      count: 0,
      now_playing: [],
    });
    await expect(telemetry.getMetrics()).resolves.toContain('# HELP');
    await expect(telemetry.getKpiMetrics()).resolves.toEqual({
      slskr_metric: { samples: [] },
    });
  });

  it('maps the documented empty now-playing response to null', async () => {
    api.get.mockResolvedValue({ data: '', status: 204 });
    await expect(nowPlaying.getNowPlaying()).resolves.toBeNull();
  });

  it.each([
    ['application state', application.getState, { data: [] }],
    ['application version', application.getVersion, { data: null }],
    ['application build', application.getBuild, { data: 'build' }],
    ['server state', server.getState, { data: [] }],
    ['destination list', destinations.getAll, { data: {} }],
    ['destination default', destinations.getDefault, { data: [] }],
    ['destination validation', destinations.validate, { data: null }],
    ['now-playing state', nowPlaying.getNowPlaying, { data: [] }],
    ['telemetry metrics', telemetry.getMetrics, { data: {} }],
    ['telemetry KPI', telemetry.getKpiMetrics, { data: [] }],
  ])('rejects malformed %s responses', async (_, helper, response) => {
    api.get.mockResolvedValue(response);
    api.post.mockResolvedValue(response);
    const promise =
      helper === destinations.validate ? helper('/downloads') : helper();
    await expect(promise).rejects.toThrow(/API returned an invalid/);
  });
});
