import api from './api';
import * as files from './files';

vi.mock('./api', () => ({
  __esModule: true,
  default: {
    delete: vi.fn(),
    get: vi.fn(),
  },
}));

describe('file API helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.get.mockResolvedValue({ data: { entries: [] } });
    api.delete.mockResolvedValue({ data: { deleted: true } });
  });

  it('encodes UTF-8 and reserved base64 characters as one path segment', async () => {
    await files.list({ root: 'downloads', subdirectory: 'Música/日本' });

    expect(api.get).toHaveBeenCalledWith(
      '/files/downloads/directories/TcO6c2ljYS%2Fml6XmnKw%3D',
    );
  });

  it('uses the same safe encoding for directory and file deletion', async () => {
    await files.deleteDirectory({ root: 'incomplete', path: '/tmp/é' });
    await files.deleteFile({ root: 'incomplete', path: '/tmp/é' });

    expect(api.delete).toHaveBeenNthCalledWith(
      1,
      '/files/incomplete/directories/L3RtcC%2FDqQ%3D%3D',
    );
    expect(api.delete).toHaveBeenNthCalledWith(
      2,
      '/files/incomplete/files/L3RtcC%2FDqQ%3D%3D',
    );
  });
});
