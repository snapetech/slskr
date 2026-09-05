import api from './api';
import { getRequests } from './quarantineJury';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('Quarantine Jury API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns the request list', async () => {
    api.get.mockResolvedValue({ data: [{ id: 'request-1' }] });

    await expect(getRequests()).resolves.toEqual([{ id: 'request-1' }]);
  });

  it('rejects malformed request lists', async () => {
    api.get.mockResolvedValue({ data: {} });

    await expect(getRequests()).rejects.toThrow(
      'Quarantine Jury API returned an invalid request list response',
    );
  });
});
