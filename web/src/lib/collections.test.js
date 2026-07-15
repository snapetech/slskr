import api from './api';
import { createShareToken, getShareManifest } from './collections';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe('share API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps reusable share tokens out of request URLs', () => {
    getShareManifest('grant/1', 'secret/token');

    expect(api.get).toHaveBeenCalledWith('/share-grants/grant%2F1/manifest', {
      headers: { 'X-Share-Token': 'secret/token' },
    });
    expect(api.get.mock.calls[0][0]).not.toContain('secret/token');
  });

  it('encodes grant ids when creating tokens', () => {
    createShareToken('grant/1', 120);

    expect(api.post).toHaveBeenCalledWith('/share-grants/grant%2F1/token', {
      expiresInSeconds: 120,
    });
  });
});
