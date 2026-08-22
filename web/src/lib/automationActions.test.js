import {
  executeDashboardRefresh,
  executeLocalDiagnostics,
  executeStaleCacheReminders,
} from './automationActions';
import * as applicationAPI from './application';
import * as libraryHealthAPI from './libraryHealth';
import * as sharesAPI from './shares';
import * as slskrAPI from './slskr';

vi.mock('./application', () => ({
  getState: vi.fn(),
}));

vi.mock('./libraryHealth', () => ({
  getIssues: vi.fn(),
}));

vi.mock('./shares', () => ({
  getAll: vi.fn(),
}));

vi.mock('./slskr', () => ({
  getSlskrStats: vi.fn(),
}));

describe('automation actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    applicationAPI.getState.mockResolvedValue({});
    libraryHealthAPI.getIssues.mockResolvedValue({ data: { issues: [] } });
    sharesAPI.getAll.mockResolvedValue({});
    slskrAPI.getSlskrStats.mockResolvedValue({});
  });

  it('runs local diagnostics through read-only backend checks', async () => {
    await expect(executeLocalDiagnostics()).resolves.toMatchObject({
      failed: 0,
      started: 1,
      summary: 'Local diagnostics completed 3/3 read-only checks; 0 failed.',
    });
    expect(applicationAPI.getState).toHaveBeenCalledOnce();
    expect(sharesAPI.getAll).toHaveBeenCalledOnce();
    expect(slskrAPI.getSlskrStats).toHaveBeenCalledOnce();
  });

  it('runs share and library reminders through read-only backend checks', async () => {
    await expect(executeStaleCacheReminders()).resolves.toMatchObject({
      failed: 0,
      started: 1,
      summary: 'Share and library reminders completed 2/2 read-only checks; 0 failed.',
    });
    expect(sharesAPI.getAll).toHaveBeenCalledOnce();
    expect(libraryHealthAPI.getIssues).toHaveBeenCalledWith({ limit: 100 });
  });

  it('reports a failed dashboard check without hiding the completed checks', async () => {
    slskrAPI.getSlskrStats.mockRejectedValue(new Error('stats unavailable'));

    await expect(executeDashboardRefresh()).resolves.toMatchObject({
      failed: 1,
      started: 1,
      summary: 'Dashboard refresh completed 2/3 read-only checks; 1 failed.',
    });
  });
});
