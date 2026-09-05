import api from './api';
import {
  getActivity,
  getAvailable,
  getJoined,
  getMessages,
  getUsers,
} from './rooms';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('room API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns validated room collections and normalized activity', async () => {
    api.get
      .mockResolvedValueOnce({ data: [{ name: 'lobby' }] })
      .mockResolvedValueOnce({ data: [{ name: 'lobby' }] })
      .mockResolvedValueOnce({
        data: { lobby: '1700000000', invalid: 'not-a-timestamp' },
      })
      .mockResolvedValueOnce({ data: [{ message: 'hello' }] })
      .mockResolvedValueOnce({ data: [{ username: 'alice' }] });

    await expect(getAvailable()).resolves.toEqual([{ name: 'lobby' }]);
    await expect(getJoined()).resolves.toEqual([{ name: 'lobby' }]);
    await expect(getActivity()).resolves.toEqual({ lobby: 1700000000 });
    await expect(getMessages({ roomName: 'lobby' })).resolves.toEqual([
      { message: 'hello' },
    ]);
    await expect(getUsers({ roomName: 'lobby' })).resolves.toEqual([
      { username: 'alice' },
    ]);
  });

  it.each([
    ['available rooms', getAvailable, {}],
    ['joined rooms', getJoined, {}],
    ['room activity', getActivity, []],
    ['room messages', () => getMessages({ roomName: 'lobby' }), {}],
    ['room users', () => getUsers({ roomName: 'lobby' }), {}],
  ])('rejects malformed %s responses', async (resource, request, data) => {
    api.get.mockResolvedValue({ data });

    await expect(request()).rejects.toThrow(
      `Rooms API returned an invalid ${resource} response`,
    );
  });

  it('encodes room message paths and optional timestamps', async () => {
    api.get.mockResolvedValue({ data: [] });

    await getMessages({ roomName: 'my room/1', since: 42 });
    await getUsers({ roomName: 'my room/1' });

    expect(api.get).toHaveBeenNthCalledWith(
      1,
      '/rooms/joined/my%20room%2F1/messages?since=42',
    );
    expect(api.get).toHaveBeenNthCalledWith(
      2,
      '/rooms/joined/my%20room%2F1/users',
    );
  });
});
