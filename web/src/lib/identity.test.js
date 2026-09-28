import api from './api';
import { addContactFromInvite, inviteContactUsername } from './identity';

vi.mock('./api', () => ({ default: { post: vi.fn() } }));

const link = (payload) => `slskdn://invite/${Buffer.from(JSON.stringify(payload)).toString('base64url')}`;
const invite = () => ({ InviteVersion: 1,
  Profile: { DisplayName: 'nodeA' },
  ExpiresAt: new Date(Date.now() + 60000).toISOString() });

describe('invite contact adapter', () => {
  beforeEach(() => vi.clearAllMocks());

  it('submits the invite username without assigning verification or permissions', () => {
    const inviteLink = link(invite());
    addContactFromInvite({ inviteLink, nickname: 'friend' });
    expect(api.post).toHaveBeenCalledWith('/contacts/from-invite', {
      inviteLink, nickname: 'friend', username: 'nodeA',
    });
  });

  it('decodes UTF-8 names', () => {
    expect(inviteContactUsername(link({ ...invite(), Profile: { DisplayName: '音楽' } }))).toBe('音楽');
  });

  it.each([
    null, '', 'https://example.org/invite', 'slskdn://invite/%%%','slskdn://invite/bm90LWpzb24',
    'slskdn://invite/' + 'a'.repeat(65536),
    link({ ...invite(), InviteVersion: 2 }),
    link({ ...invite(), ExpiresAt: 'invalid' }),
    link({ ...invite(), ExpiresAt: '2020-01-01T00:00:00Z' }),
    link({ ...invite(), Profile: { DisplayName: '' } }),
    link({ ...invite(), Profile: { DisplayName: 'a\nb' } }),
    link({ ...invite(), Profile: { DisplayName: 'a'.repeat(256) } }),
  ])('rejects malformed, expired, or oversized invites before posting', (inviteLink) => {
    expect(() => addContactFromInvite({ inviteLink, nickname: 'friend' })).toThrow();
    expect(api.post).not.toHaveBeenCalled();
  });
});
