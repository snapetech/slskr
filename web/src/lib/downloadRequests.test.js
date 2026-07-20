import api from './api';
import * as downloadRequests from './downloadRequests';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    patch: vi.fn(),
    post: vi.fn(),
  },
}));

describe('downloadRequests', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('lists, gets, renames, and cancels download requests', async () => {
    api.get.mockResolvedValue({ data: [] });
    api.patch.mockResolvedValue({ data: { id: 'request/1', name: 'Renamed' } });
    api.post.mockResolvedValue({});
    await downloadRequests.list({ state: 'Pending' });
    await downloadRequests.get('request/1');
    await downloadRequests.rename('request/1', 'Renamed');
    await downloadRequests.cancel('request/1');
    expect(api.get).toHaveBeenNthCalledWith(1, '/downloads/requests?state=Pending');
    expect(api.get).toHaveBeenNthCalledWith(2, '/downloads/requests/request%2F1');
    expect(api.patch).toHaveBeenCalledWith('/downloads/requests/request%2F1/name', {
      name: 'Renamed',
    });
    expect(api.post).toHaveBeenCalledWith('/downloads/requests/request%2F1/cancel');
  });
});
