import api from './api';
import { getSourceProviders } from './sourceProviders';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('source provider API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('normalizes the provider catalog while preserving compatibility casing', async () => {
    api.get.mockResolvedValue({
      data: {
        AcquisitionPlanningEnabled: true,
        ProfilePolicies: [{ profileId: 'lossless-exact' }],
        Providers: [{ id: 'Soulseek' }],
      },
    });

    await expect(getSourceProviders()).resolves.toEqual({
      acquisitionPlanningEnabled: true,
      profilePolicies: [{ profileId: 'lossless-exact' }],
      providers: [{ id: 'Soulseek' }],
    });
  });

  it.each([
    ['catalog', []],
    ['profile policies', { acquisitionPlanningEnabled: true, providers: [] }],
    ['providers', { acquisitionPlanningEnabled: true, profilePolicies: [] }],
    [
      'acquisition-planning flag',
      {
        acquisitionPlanningEnabled: 'yes',
        profilePolicies: [],
        providers: [],
      },
    ],
  ])('rejects malformed %s responses', async (resource, data) => {
    api.get.mockResolvedValue({ data });

    await expect(getSourceProviders()).rejects.toThrow(
      resource === 'catalog'
        ? 'Source providers API returned an invalid catalog response'
        : resource === 'acquisition-planning flag'
          ? 'Source providers API returned an invalid acquisition-planning flag'
          : `Source providers API returned an invalid ${resource} response`,
    );
  });
});
