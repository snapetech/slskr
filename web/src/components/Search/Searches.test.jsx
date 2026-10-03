// <copyright file="Searches.test.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import Searches from './Searches';
import { createSearchHubConnection } from '../../lib/hubFactory';
import { getCapabilities } from '../../lib/slskr';
import * as library from '../../lib/searches';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import {
  MemoryRouter,
  Route,
  Routes,
} from 'react-router-dom';

vi.mock('../../lib/hubFactory', () => ({
  createSearchHubConnection: vi.fn(),
}));
vi.mock('../../lib/slskr', () => ({
  getCapabilities: vi.fn(),
}));
vi.mock('../../lib/searches', () => ({
  create: vi.fn(),
  getAll: vi.fn(),
  getStatus: vi.fn(),
  remove: vi.fn(),
  removeAll: vi.fn(),
  stop: vi.fn(),
}));
vi.mock('./AlbumCompletionPanel', () => ({ default: () => null }));
vi.mock('./ArtistReleaseRadarPanel', () => ({ default: () => null }));
vi.mock('./DiscoveryGraphAtlasPanel', () => ({ default: () => null }));
vi.mock('./FederatedTasteRecommendationsPanel', () => ({ default: () => null }));
vi.mock('./MusicBrainzLookup', () => ({ default: () => null }));
vi.mock('./SongIDPanel', () => ({
  default: () => <div data-testid="songid-panel">SongID panel</div>,
}));
vi.mock('./Detail/SearchDetail', () => ({ default: () => null }));
vi.mock('./List/SearchList', () => ({
  default: ({ onStop, searches }) => {
    const search = Object.values(searches ?? {})[0];
    if (!search) {
      return null;
    }

    return (
      <>
        <span data-testid="search-list-search-text">{search.searchText}</span>
        <span data-testid="search-list-file-count">{search.fileCount}</span>
        <button
          onClick={() =>
            onStop({
              fileCount: 1,
              id: search.id,
              searchText: 'stale search row',
              state: 'InProgress',
            })
          }
          type="button"
        >
          Stop search
        </button>
      </>
    );
  },
}));

const callbacks = {};

const renderSearches = async ({
  hubStart,
  initialEntries = ['/searches'],
  runtimeProfile,
  waitForInput = true,
} = {}) => {
  callbacks.list = undefined;
  createSearchHubConnection.mockReturnValue({
    on: vi.fn((eventName, callback) => {
      callbacks[eventName] = callback;
    }),
    onclose: vi.fn(),
    onreconnected: vi.fn(),
    onreconnecting: vi.fn(),
    start: vi.fn(() => {
      if (hubStart) return hubStart();
      callbacks.list?.([]);
      return Promise.resolve();
    }),
    stop: vi.fn(),
  });

  const searches = (
    <Searches
      runtimeProfile={runtimeProfile}
      server={{ isConnected: true }}
    />
  );

  const view = render(
    <MemoryRouter initialEntries={initialEntries}>
      <Routes>
        <Route path="/searches" element={searches} />
        <Route path="/searches/:id" element={searches} />
      </Routes>
    </MemoryRouter>,
  );

  if (waitForInput) return screen.findByTestId('search-input');
  return view;
};

describe('Searches', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    getCapabilities.mockResolvedValue({ features: [] });
    library.create.mockResolvedValue({});
    library.getAll.mockResolvedValue([]);
    library.getStatus.mockResolvedValue({});
  });

  it('loads existing searches from REST', async () => {
    library.getAll.mockResolvedValue([
      {
        id: 'search-1',
        searchText: 'existing search',
        startedAt: '2026-05-15T00:00:00Z',
      },
    ]);

    await renderSearches();

    await waitFor(() => expect(library.getAll).toHaveBeenCalledTimes(1));
  });

  it('renders REST results while the search updates hub connection is pending', async () => {
    let resolveHub;
    const pendingHubStart = new Promise((resolve) => {
      resolveHub = resolve;
    });
    library.getAll.mockResolvedValue([
      {
        id: 'search-1',
        searchText: 'results before the event stream',
        startedAt: '2026-05-15T00:00:00Z',
      },
    ]);

    await renderSearches({
      hubStart: () => pendingHubStart,
      waitForInput: false,
    });

    expect(await screen.findByTestId('search-list-search-text'))
      .toBeInTheDocument();
    expect(screen.getByTestId('search-list-search-text'))
      .toHaveTextContent('results before the event stream');
    expect(screen.getByText(/connecting to live search updates/i))
      .toBeInTheDocument();

    await act(async () => {
      resolveHub();
      await pendingHubStart;
    });
    await waitFor(() => expect(screen.queryByText(/connecting to live search updates/i))
      .not.toBeInTheDocument());
  });

  it('keeps loaded search results visible when the updates hub fails', async () => {
    library.getAll.mockResolvedValue([
      {
        id: 'search-1',
        searchText: 'last loaded search',
        startedAt: '2026-05-15T00:00:00Z',
      },
    ]);

    await renderSearches({
      hubStart: () => Promise.reject(new Error('search hub unavailable')),
      waitForInput: false,
    });

    expect(await screen.findByTestId('search-list-search-text'))
      .toHaveTextContent('last loaded search');
    expect(await screen.findByText(/search hub unavailable/i)).toBeInTheDocument();
  });

  it('clears a REST load warning when the hub later supplies valid search data', async () => {
    library.getAll.mockRejectedValue(new Error('REST search list unavailable'));

    await renderSearches({ hubStart: () => Promise.resolve(), waitForInput: false });
    expect(await screen.findByText(/REST search list unavailable/i))
      .toBeInTheDocument();

    await act(async () => {
      callbacks.list?.([
        {
          id: 'search-1',
          searchText: 'live hub recovery',
          startedAt: '2026-05-15T00:00:00Z',
        },
      ]);
    });

    expect(await screen.findByTestId('search-list-search-text'))
      .toHaveTextContent('live hub recovery');
    expect(screen.queryByText(/REST search list unavailable/i))
      .not.toBeInTheDocument();
  });

  it('does not report a REST failure when the hub list succeeds first', async () => {
    let rejectRest;
    library.getAll.mockImplementation(() => new Promise((_resolve, reject) => {
      rejectRest = reject;
    }));

    await renderSearches({ hubStart: () => Promise.resolve(), waitForInput: false });
    await waitFor(() => expect(rejectRest).toBeTypeOf('function'));

    await act(async () => {
      callbacks.list?.([
        {
          id: 'search-1',
          searchText: 'hub wins the race',
          startedAt: '2026-05-15T00:00:00Z',
        },
      ]);
    });
    await act(async () => {
      rejectRest(new Error('late REST failure'));
    });

    expect(await screen.findByTestId('search-list-search-text'))
      .toHaveTextContent('hub wins the race');
    expect(screen.queryByText(/late REST failure/i)).not.toBeInTheDocument();
  });

  it('hydrates native search state from the REST list instead of relying on hub history', async () => {
    library.getAll.mockResolvedValue([
      {
        id: 'search-1',
        searchText: 'native search',
        startedAt: '2026-05-15T00:00:00Z',
      },
    ]);

    await renderSearches({
      initialEntries: ['/searches/search-1'],
      runtimeProfile: 'native',
      waitForInput: false,
    });

    await waitFor(() => expect(library.getAll).toHaveBeenCalledTimes(1));
    expect(library.getStatus).not.toHaveBeenCalled();
  });

  it('loads a detail record when the initial search list races navigation', async () => {
    library.getAll.mockResolvedValue([]);
    library.getStatus.mockResolvedValue({
      id: 'search-1',
      searchText: 'raced search',
      state: 'Completed',
    });

    await renderSearches({
      initialEntries: ['/searches/search-1'],
      runtimeProfile: 'native',
      waitForInput: false,
    });

    await waitFor(() =>
      expect(library.getStatus).toHaveBeenCalledWith(
        expect.objectContaining({ id: 'search-1', signal: expect.any(AbortSignal) }),
      ),
    );
  });

  it('aborts a missing-status request when its detail route unmounts', async () => {
    let requestSignal;
    library.getAll.mockResolvedValue([]);
    library.getStatus.mockImplementation(({ signal }) => {
      requestSignal = signal;
      return new Promise((_resolve, reject) => {
        signal.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
      });
    });

    const view = await renderSearches({
      initialEntries: ['/searches/search-1'],
      runtimeProfile: 'native',
      waitForInput: false,
    });
    await waitFor(() => expect(library.getStatus).toHaveBeenCalled());
    view.unmount();

    expect(requestSignal.aborted).toBe(true);
  });

  it('preserves the freshest search projection when stop receives a stale row', async () => {
    library.getAll.mockResolvedValue([
      {
        fileCount: 9,
        id: 'search-1',
        responseCount: 4,
        searchText: 'fresh search',
        state: 'InProgress',
      },
    ]);
    library.stop.mockResolvedValue({});

    await renderSearches();

    await waitFor(() =>
      expect(screen.getByTestId('search-list-file-count')).toHaveTextContent('9'),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Stop search' }));

    await waitFor(() => expect(library.stop).toHaveBeenCalledWith({ id: 'search-1' }));
    expect(screen.getByTestId('search-list-file-count')).toHaveTextContent('9');
  });

  it('refreshes generic search events by resource id', async () => {
    library.getStatus.mockResolvedValue({
      id: 'search-1',
      searchText: 'event search',
      startedAt: '2026-05-15T00:00:00Z',
    });

    await renderSearches();
    callbacks.create?.({
      id: 42,
      kind: 'search.started',
      resource: 'search-1',
    });

    await waitFor(() =>
      expect(library.getStatus).toHaveBeenCalledWith({ id: 'search-1' }),
    );
  });

  it('accepts compatibility searchId events', async () => {
    library.getStatus.mockResolvedValue({
      id: 'search-1',
      searchText: 'compatibility search',
      startedAt: '2026-05-15T00:00:00Z',
    });

    await renderSearches();
    callbacks.create?.({ id: 42, searchId: 'search-1' });

    await waitFor(() =>
      expect(library.getStatus).toHaveBeenCalledWith({ id: 'search-1' }),
    );
  });

  it('keeps ScenePodBridge disabled by default and creates ordinary searches without providers', async () => {
    const input = await renderSearches();

    expect(screen.queryByText('Search Sources:')).not.toBeInTheDocument();

    fireEvent.change(input, { target: { value: 'beatles' } });
    fireEvent.keyUp(input, { key: 'Enter' });

    await waitFor(() => expect(library.create).toHaveBeenCalledTimes(1));
    expect(library.create).toHaveBeenCalledWith(
      expect.objectContaining({
        acquisitionProfile: 'lossless-exact',
        providers: null,
        searchText: 'beatles',
      }),
    );
  });

  it('hydrates the newly created search before displaying its result count', async () => {
    library.getStatus.mockResolvedValue({
      fileCount: 3,
      responseCount: 1,
      responsesAvailable: true,
      searchText: 'beatles',
      state: 'InProgress',
      status: 'active',
    });

    const input = await renderSearches();
    fireEvent.change(input, { target: { value: 'beatles' } });
    fireEvent.keyUp(input, { key: 'Enter' });

    await waitFor(() =>
      expect(screen.getByTestId('search-list-file-count')).toHaveTextContent('3'),
    );
    expect(library.getStatus).toHaveBeenCalledWith({ id: expect.any(String) });
  });

  it('only sends bridge providers when the backend explicitly advertises ScenePodBridge', async () => {
    getCapabilities.mockResolvedValue({
      feature: { scenePodBridge: true },
      features: ['scene_pod_bridge'],
    });
    const input = await renderSearches();

    expect(await screen.findByText('Search Sources:')).toBeInTheDocument();

    fireEvent.change(input, { target: { value: 'beatles' } });
    fireEvent.keyUp(input, { key: 'Enter' });

    await waitFor(() => expect(library.create).toHaveBeenCalledTimes(1));
    expect(library.create).toHaveBeenCalledWith(
      expect.objectContaining({
        acquisitionProfile: 'lossless-exact',
        providers: ['pod', 'scene'],
        searchText: 'beatles',
      }),
    );
  });

  it('defaults secondary search sections closed and remembers expanded state', async () => {
    await renderSearches();

    expect(screen.getByRole('button', { name: 'Expand SongID' })).toBeInTheDocument();
    expect(screen.queryByTestId('songid-panel')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Expand SongID' }));

    expect(screen.getByTestId('songid-panel')).toBeInTheDocument();
    expect(localStorage.getItem('slskr.search.section.songid')).toBe('open');
  });

  it('places Search Results directly after the search form and before discovery tools', async () => {
    await renderSearches();

    const headings = screen
      .getAllByRole('heading', { level: 4 })
      .map((heading) => heading.textContent);

    expect(headings.slice(0, 3)).toEqual(['Search', 'Search Results', 'SongID']);
  });

  it('shows and persists the selected acquisition profile', async () => {
    await renderSearches();

    expect(screen.getByText('Acquisition Profile')).toBeInTheDocument();
    expect(screen.getAllByText('Lossless Exact').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByTestId('acquisition-profile-select'));
    fireEvent.click(screen.getByText('Conservative Network'));

    expect(localStorage.getItem('slskr.acquisitionProfile')).toBe(
      'conservative-network',
    );
    expect(
      screen.getAllByText('Lower concurrency, no automatic public-peer retries.')
        .length,
    ).toBeGreaterThan(0);

    const input = screen.getByTestId('search-input');
    fireEvent.change(input, { target: { value: 'rare live set' } });
    fireEvent.keyUp(input, { key: 'Enter' });

    await waitFor(() => expect(library.create).toHaveBeenCalledTimes(1));
    expect(library.create).toHaveBeenCalledWith(
      expect.objectContaining({
        acquisitionProfile: 'conservative-network',
        searchText: 'rare live set',
      }),
    );
  });

  it('uses stored collapsed state for primary search sections', async () => {
    localStorage.setItem('slskr.search.section.search', 'closed');

    await renderSearches({ waitForInput: false });

    expect(screen.getByRole('button', { name: 'Expand Search' })).toBeInTheDocument();
    expect(screen.queryByTestId('search-input')).not.toBeInTheDocument();
  });

  it('keeps manual search out of acquisition review', async () => {
    await renderSearches();

    expect(
      screen.queryByRole('button', { name: 'Add search phrase to Discovery Inbox' }),
    ).not.toBeInTheDocument();
  });
});
