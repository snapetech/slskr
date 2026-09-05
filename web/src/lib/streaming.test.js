import api from './api';
import {
  buildDirectStreamUrl,
  buildPeerStreamUrl,
  buildTicketedStreamUrl,
  createShareStreamTicket,
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
