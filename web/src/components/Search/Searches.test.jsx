// <copyright file="Searches.test.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import Searches from './Searches';
import { createSearchHubConnection } from '../../lib/hubFactory';
import { getCapabilities } from '../../lib/slskr';
import * as library from '../../lib/searches';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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
    start: vi.fn(async () => {
      callbacks.list?.([]);
    }),
    stop: vi.fn(),
  });

  const searches = (
    <Searches
      runtimeProfile={runtimeProfile}
      server={{ isConnected: true }}
    />
  );

  render(
    <MemoryRouter initialEntries={initialEntries}>
      <Routes>
        <Route path="/searches" element={searches} />
        <Route path="/searches/:id" element={searches} />
      </Routes>
    </MemoryRouter>,
  );

  return waitForInput ? screen.findByTestId('search-input') : undefined;
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

  it('loads existing searches after the update stream connects', async () => {
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
      expect(library.getStatus).toHaveBeenCalledWith({ id: 'search-1' }),
    );
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
