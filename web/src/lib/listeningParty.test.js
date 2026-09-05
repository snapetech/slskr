import api from './api';
import { getPartyDirectory, getPartyState } from './listeningParty';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('listening party API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns directory entries and handles an empty party state', async () => {
    api.get
      .mockResolvedValueOnce({ data: [{ partyId: 'party-1' }] })
      .mockResolvedValueOnce({ status: 204, data: undefined })
      .mockResolvedValueOnce({ status: 200, data: { partyId: 'party-1' } });

    await expect(getPartyDirectory()).resolves.toEqual([
      { partyId: 'party-1' },
    ]);
    await expect(getPartyState('pod-1', 'channel-1')).resolves.toBeNull();
    await expect(getPartyState('pod-1', 'channel-1')).resolves.toEqual({
      partyId: 'party-1',
    });
  });

  it('rejects malformed directory and state responses', async () => {
    api.get
      .mockResolvedValueOnce({ data: {} })
      .mockResolvedValueOnce({ status: 200, data: [] });

    await expect(getPartyDirectory()).rejects.toThrow(
      'Listening party API returned an invalid party directory response',
    );
    await expect(getPartyState('pod-1', 'channel-1')).rejects.toThrow(
      'Listening party API returned an invalid party state response',
    );
  });
});
