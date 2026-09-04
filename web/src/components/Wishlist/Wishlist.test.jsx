import Wishlist from './Wishlist';
import * as spotifyIntegrationAPI from '../../lib/spotifyIntegration';
import * as wishlistAPI from '../../lib/wishlist';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('../../lib/wishlist', () => ({
  create: vi.fn(),
  getIgnoredResults: vi.fn(),
  getAll: vi.fn(),
  importCsv: vi.fn(),
  removeIgnoredResult: vi.fn(),
  remove: vi.fn(),
  runSearch: vi.fn(),
  update: vi.fn(),
  updateFilters: vi.fn(),
}));

vi.mock('../../lib/spotifyIntegration', () => ({
  disconnectSpotify: vi.fn(),
  getSpotifyStatus: vi.fn(),
  startSpotifyAuthorization: vi.fn(),
}));

const renderWishlist = () =>
  render(
    <MemoryRouter>
      <Wishlist />
    </MemoryRouter>,
  );

describe('Wishlist', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    spotifyIntegrationAPI.getSpotifyStatus.mockResolvedValue({
      configured: false,
      connected: false,
    });
    wishlistAPI.getAll.mockResolvedValue([
      {
        autoDownload: false,
        enabled: true,
        filter: 'flac',
        id: 'wish-1',
        lastMatchCount: 0,
        lastSearchedAt: null,
        searchText: 'rare album',
        totalSearchCount: 0,
      },
      {
        autoDownload: true,
        enabled: true,
        id: 'wish-2',
        lastMatchCount: 3,
        lastSearchedAt: '2026-04-30T19:30:00Z',
        searchText: 'auto track',
        totalSearchCount: 2,
      },
    ]);
    wishlistAPI.getIgnoredResults.mockResolvedValue([]);
    wishlistAPI.removeIgnoredResult.mockResolvedValue(undefined);
  });

  it('shows unified request states for wishlist rows', async () => {
    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    expect(screen.getByText('Wanted')).toBeInTheDocument();
    expect(screen.getAllByText('Automatic').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText('Request Portal Summary')).toBeInTheDocument();
    expect(screen.getByText('23 left')).toBeInTheDocument();
  });

  it('keeps wishlist rows on direct request states without inbox promotion', async () => {
    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    expect(screen.queryByTitle('Send to Discovery Inbox review')).not.toBeInTheDocument();
    expect(screen.getByText('Wanted')).toBeInTheDocument();
  });

  it('copies a wishlist request review packet', async () => {
    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Copy Wishlist request review' }));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        expect.stringContaining('slskr Wishlist request review'),
      );
    });
  });

  it('runs enabled wishlist searches in a bounded batch', async () => {
    wishlistAPI.runSearch.mockResolvedValue({ responseCount: 4 });

    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Run enabled Wishlist searches' }));

    await waitFor(() => {
      expect(wishlistAPI.runSearch).toHaveBeenCalledTimes(2);
    });
    expect(wishlistAPI.runSearch).toHaveBeenCalledWith('wish-1');
    expect(wishlistAPI.runSearch).toHaveBeenCalledWith('wish-2');
    expect(screen.getByText(/Ran 2 enabled Wishlist searches/)).toBeInTheDocument();
  });

  it('loads and restores ignored folders from the wishlist editor', async () => {
    wishlistAPI.getIgnoredResults.mockResolvedValue([
      {
        directory: 'Artist/Album',
        id: 'ignored-1',
        username: 'peer',
      },
    ]);

    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    fireEvent.click(screen.getAllByTitle('Edit')[0]);

    expect(await screen.findByText('Ignored Result Folders')).toBeInTheDocument();
    expect(wishlistAPI.getIgnoredResults).toHaveBeenCalledWith('wish-1');

    fireEvent.click(
      screen.getByRole('button', {
        name: 'Restore ignored folder Artist/Album',
      }),
    );

    await waitFor(() =>
      expect(wishlistAPI.removeIgnoredResult).toHaveBeenCalledWith(
        'wish-1',
        'ignored-1',
      ),
    );
    await waitFor(() =>
      expect(screen.getByText('No folders are ignored.')).toBeInTheDocument(),
    );
  });

  it('bulk-edits filters for selected wishlist items', async () => {
    wishlistAPI.updateFilters.mockResolvedValue({ updatedCount: 2 });

    renderWishlist();

    expect(await screen.findByText('rare album')).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole('checkbox', { name: 'Select rare album for bulk actions' }),
    );
    fireEvent.click(
      screen.getByRole('checkbox', { name: 'Select auto track for bulk actions' }),
    );
    fireEvent.change(screen.getByLabelText('Bulk wishlist filter'), {
      target: { value: 'mp3 minbr:320' },
    });
    fireEvent.click(
      screen.getByRole('button', { name: 'Apply filter to selected wishlist items' }),
    );

    await waitFor(() => {
      expect(wishlistAPI.updateFilters).toHaveBeenCalledWith(
        expect.arrayContaining(['wish-1', 'wish-2']),
        'mp3 minbr:320',
      );
    });
  });

  it('encodes search ids and hides invalid dates', async () => {
    wishlistAPI.getAll.mockResolvedValue([
      {
        autoDownload: false,
        enabled: true,
        id: 'wish-special',
        lastMatchCount: 1,
        lastSearchId: 'search/with?chars%',
        lastSearchedAt: 'not-a-date',
        searchText: 'encoded route',
        totalSearchCount: 1,
      },
    ]);

    renderWishlist();

    expect(await screen.findByText('encoded route')).toBeInTheDocument();
    expect(screen.getByText('Never')).toBeInTheDocument();
    expect(screen.getByTitle('View last search results').closest('a')).toHaveAttribute(
      'href',
      '/searches/search%2Fwith%3Fchars%25',
    );
  });
});
