import api from './api';
import * as users from './users';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe('users browse', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('returns a completed cached browse without dispatching another request', async () => {
    api.get
      .mockResolvedValueOnce({ data: { isComplete: true } })
      .mockResolvedValueOnce({ data: { directories: [], lockedDirectories: [] } });

    await expect(users.browse({ username: 'peer one' })).resolves.toEqual({
      directories: [],
      lockedDirectories: [],
    });
    expect(api.get).toHaveBeenNthCalledWith(
      1,
      '/users/peer%20one/browse/status',
    );
    expect(api.get).toHaveBeenNthCalledWith(2, '/users/peer%20one/browse');
    expect(api.post).not.toHaveBeenCalled();
  });

  it('dispatches and waits for an asynchronous browse projection', async () => {
    api.get
      .mockResolvedValueOnce({ data: { state: 'NotStarted' } })
      .mockResolvedValueOnce({ data: { state: 'Completed', isComplete: true } })
      .mockResolvedValueOnce({ data: { directories: [{ name: 'Music' }] } });
    api.post.mockResolvedValue({ data: { status: 'requested' } });

    await expect(users.browse({ username: 'friend' })).resolves.toEqual({
      directories: [{ name: 'Music' }],
    });
    expect(api.post).toHaveBeenCalledWith('/users/friend/browse/request');
    expect(api.get).toHaveBeenLastCalledWith('/users/friend/browse');
  });

  it('surfaces the bounded daemon reason for unavailable peers', async () => {
    api.get
      .mockResolvedValueOnce({ data: { state: 'NotStarted' } })
      .mockResolvedValueOnce({
        data: { state: 'Failed', reason: 'browse failed' },
      });
    api.post.mockResolvedValue({ data: { status: 'requested' } });

    await expect(users.browse({ username: 'offline' })).rejects.toThrow(
      'browse failed',
    );
  });
});
