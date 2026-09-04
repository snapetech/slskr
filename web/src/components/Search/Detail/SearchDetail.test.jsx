import SearchDetail from './SearchDetail';
import { getResponses } from '../../../lib/searches';
import * as wishlistAPI from '../../../lib/wishlist';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';

vi.mock('semantic-ui-react', async () => {
  const actual = await vi.importActual('semantic-ui-react');
  return {
    ...actual,
    Input: ({ action, ...props }) => (
      <div>
        <input {...props} />
        {action}
      </div>
    ),
  };
});

vi.mock('../../../lib/albumCandidatePicker', () => ({
  buildAlbumCandidates: vi.fn(() => []),
  getAlbumCandidateFilter: vi.fn(),
}));
vi.mock('../../../lib/albumDecisionRules', () => ({
  saveAlbumDecisionRule: vi.fn(),
}));
vi.mock('../../../lib/discoveryGraph', () => ({
  buildDiscoveryGraph: vi.fn(),
}));
vi.mock('../../../lib/searchCandidateRanking', () => ({
  rankSearchResponses: vi.fn(({ responses }) => responses),
}));
vi.mock('../../../lib/searchResultDeduplication', () => ({
  deduplicateSearchResponses: vi.fn(({ responses }) => ({
    foldedCount: 0,
    responses,
  })),
}));
vi.mock('../../../lib/searches', () => ({
  blockUser: vi.fn(() => []),
  createBatch: vi.fn(),
  filterResponse: vi.fn(({ response }) => response),
  getBlockedUsers: vi.fn(() => []),
  getResponses: vi.fn(),
  getUserDownloadStats: vi.fn(async () => ({})),
  parseFiltersFromString: vi.fn(() => []),
  unblockUser: vi.fn(() => []),
}));
vi.mock('../../../lib/storage', () => ({
  getLocalStorageItem: vi.fn((_key, fallback = '') => fallback),
  removeLocalStorageItem: vi.fn(),
  setLocalStorageItem: vi.fn(),
}));
vi.mock('../../../lib/userNotes', () => ({
  getAllNotes: vi.fn(async () => ({ data: [] })),
}));
vi.mock('../../../lib/util', () => ({
  getDirectoryName: vi.fn((filename) =>
    String(filename).replaceAll('\\', '/').split('/').slice(0, -1).join('/'),
  ),
  sleep: vi.fn(async () => undefined),
}));
vi.mock('../../../lib/wishlist', () => ({
  getIgnoredResults: vi.fn(),
  ignoreResult: vi.fn(),
}));
vi.mock('../../Shared/ErrorSegment', () => ({ default: () => null }));
vi.mock('../../Shared/LoaderSegment', () => ({ default: () => null }));
vi.mock('../../Shared/Switch', () => ({
  default: ({ children }) => <>{children}</>,
}));
vi.mock('../DiscoveryGraphModal', () => ({ default: () => null }));
vi.mock('../Response', () => ({
  default: ({ onIgnoreDirectory, response }) => (
    <div data-testid="search-response">
      <span data-testid="visible-files">
        {response.files.map((file) => file.filename).join(',')}
      </span>
      {onIgnoreDirectory && (
        <button
          onClick={() => onIgnoreDirectory('Album')}
          type="button"
        >
          Ignore folder
        </button>
      )}
    </div>
  ),
}));
vi.mock('./SearchDetailHeader', () => ({ default: () => null }));
vi.mock('./SearchFilterModal', () => ({ default: () => null }));

const response = {
  fileCount: 3,
  files: [
    { filename: 'Album\\keep.mp3', size: 1 },
    { filename: 'Album\\ignored.mp3', size: 1 },
    { filename: 'Other\\allowed.mp3', size: 1 },
  ],
  hasFreeUploadSlot: true,
  lockedFileCount: 0,
  lockedFiles: [],
  username: 'PeerOne',
};

const createProps = (searchOverrides = {}) => ({
  creating: false,
  disabled: false,
  onCreate: vi.fn(),
  onRemove: vi.fn(),
  onStop: vi.fn(),
  removing: false,
  search: {
    fileCount: response.fileCount,
    id: 'search-id',
    isComplete: true,
    lockedFileCount: 0,
    responseCount: 1,
    searchText: 'test',
    state: 'Complete',
    ...searchOverrides,
  },
  stopping: false,
});

describe('SearchDetail wishlist folder ignores', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getResponses.mockResolvedValue([response]);
    wishlistAPI.getIgnoredResults.mockResolvedValue([]);
    wishlistAPI.ignoreResult.mockResolvedValue({
      directory: 'Album',
      id: 'ignored-1',
      username: 'PeerOne',
    });
  });

  it('loads and filters persisted wishlist folder ignores', async () => {
    wishlistAPI.getIgnoredResults.mockResolvedValue([
      {
        directory: 'album/',
        id: 'ignored-1',
        username: 'peerone',
      },
    ]);

    render(
      <SearchDetail
        {...createProps({ wishlistItemId: 'wishlist-1' })}
      />,
    );

    await waitFor(() =>
      expect(screen.getByTestId('visible-files')).toHaveTextContent(
        'Other\\allowed.mp3',
      ),
    );
    expect(screen.getByTestId('visible-files')).not.toHaveTextContent('Album');
    expect(wishlistAPI.getIgnoredResults).toHaveBeenCalledWith('wishlist-1');
  });

  it('retries completed result hydration when rows lag the terminal projection', async () => {
    getResponses
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([response]);

    render(<SearchDetail {...createProps()} />);

    await waitFor(() =>
      expect(screen.getByTestId('visible-files')).toHaveTextContent(
        'Album\\keep.mp3,Album\\ignored.mp3,Other\\allowed.mp3',
      ),
    );
    expect(getResponses).toHaveBeenCalledTimes(2);
  });

  it('confirms and persists a new wishlist folder ignore', async () => {
    render(
      <SearchDetail
        {...createProps({ wishlistItemId: 'wishlist-1' })}
      />,
    );

    await screen.findByRole('button', { name: 'Ignore folder' });
    fireEvent.click(screen.getByRole('button', { name: 'Ignore folder' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Ignore Folder' }));

    await waitFor(() =>
      expect(wishlistAPI.ignoreResult).toHaveBeenCalledWith('wishlist-1', {
        directory: 'Album',
        username: 'PeerOne',
      }),
    );
  });

  it('does not expose folder-ignore controls for ordinary searches', async () => {
    render(<SearchDetail {...createProps()} />);

    expect(await screen.findByRole('note')).toHaveTextContent(
      'Folder ignores are available only for wishlist searches',
    );
    expect(screen.queryByRole('button', { name: 'Ignore folder' })).not.toBeInTheDocument();
    expect(wishlistAPI.getIgnoredResults).not.toHaveBeenCalled();
  });
});
