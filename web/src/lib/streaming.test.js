import api from './api';
import {
  buildDirectStreamUrl,
  buildPeerStreamUrl,
  buildTicketedStreamUrl,
  createPeerStreamTicket,
  createShareStreamTicket,
  createStreamTicket,
} from './streaming';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    post: vi.fn(),
  },
}));

describe('share streaming helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns validated stream ticket responses', async () => {
    api.post.mockResolvedValueOnce({ data: { ticket: 'stream-ticket' } });
    await expect(createStreamTicket('content/1')).resolves.toBe('stream-ticket');

    api.post.mockResolvedValueOnce({
      data: { streamUrl: '/api/v0/peer-streams/ticket', ticket: 'peer-ticket' },
    });
    await expect(
      createPeerStreamTicket({ filename: 'song.flac', size: 42, username: 'peer' }),
    ).resolves.toEqual({
      streamUrl: '/api/v0/peer-streams/ticket',
      ticket: 'peer-ticket',
    });
  });

  it.each([
    ['stream ticket', createStreamTicket, { data: {} }],
    ['share stream ticket', createShareStreamTicket, { data: { ticket: '' } }],
    ['peer ticket', createPeerStreamTicket, { data: { ticket: 'ticket' } }],
  ])('rejects malformed %s responses', async (_, helper, response) => {
    api.post.mockResolvedValue(response);
    const promise =
      helper === createStreamTicket
        ? helper('content/1')
        : helper === createShareStreamTicket
          ? helper('content/1', 'share-token')
          : helper({ filename: 'song.flac', size: 42, username: 'peer' });
    await expect(promise).rejects.toThrow('Streaming API returned an invalid');
  });

  it('exchanges a header token for a short-lived stream ticket', async () => {
    api.post.mockResolvedValue({ data: { ticket: 'opaque-ticket' } });

    await expect(
      createShareStreamTicket('content/1', 'reusable-secret'),
    ).resolves.toBe('opaque-ticket');
    expect(api.post).toHaveBeenCalledWith(
      '/streams/content%2F1/share-ticket',
      undefined,
      { headers: { 'X-Share-Token': 'reusable-secret' } },
    );
    expect(api.post.mock.calls[0][0]).not.toContain('reusable-secret');
  });

  it('builds stream URLs from the normalized API base', () => {
    expect(buildDirectStreamUrl('content/1')).toContain('/api/v0/streams/content%2F1');
    expect(buildTicketedStreamUrl('content/1', 'ticket/1')).toContain(
      '?ticket=ticket%2F1',
    );
    expect(buildPeerStreamUrl('api/v0/streams/content')).toMatch(
      /\/api\/v0\/streams\/content$/,
    );
  });
});
