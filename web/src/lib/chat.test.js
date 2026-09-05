import api from './api';
import {
  get,
  getAll,
  hasUnAcknowledgedMessages,
} from './chat';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
  },
}));

describe('chat API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns validated conversation and unread activity responses', async () => {
    const conversation = {
      hasUnAcknowledgedMessages: true,
      isActive: true,
      messages: [{ id: 1, message: 'hello' }],
      unAcknowledgedMessageCount: 1,
      username: 'peer',
    };
    api.get
      .mockResolvedValueOnce({ data: [conversation] })
      .mockResolvedValueOnce({ data: conversation })
      .mockResolvedValueOnce({ data: true });

    await expect(getAll({ unAcknowledgedOnly: true })).resolves.toEqual([
      conversation,
    ]);
    await expect(get({ username: 'peer', since: 10 })).resolves.toEqual(
      conversation,
    );
    await expect(hasUnAcknowledgedMessages()).resolves.toBe(true);
    expect(api.get).toHaveBeenNthCalledWith(
      2,
      '/conversations/peer?since=10',
    );
  });

  it.each([
    ['conversation list', 'getAll', { data: {} }],
    ['conversation list entry', 'getAll', { data: [null] }],
    ['conversation detail', 'get', { data: [] }],
    [
      'conversation messages',
      'get',
      { data: { messages: {} } },
    ],
    [
      'unread activity',
      'hasUnAcknowledgedMessages',
      { data: 'true' },
    ],
  ])('rejects malformed %s responses', async (_, helper, response) => {
    api.get.mockResolvedValue(response);

    const promise =
      helper === 'getAll'
        ? getAll()
        : helper === 'get'
          ? get({ username: 'peer' })
          : hasUnAcknowledgedMessages();
    await expect(promise).rejects.toThrow('Chat API returned an invalid');
  });
});
