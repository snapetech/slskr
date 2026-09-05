// <copyright file="index.test.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as mediacore from '../../../lib/mediacore';
import MediaCore from './index';
import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/mediacore', () => ({
  getConflictStrategies: vi.fn(),
  getContentIdStats: vi.fn(),
  getContentOpinions: vi.fn(),
  getChannels: vi.fn(),
  searchContent: vi.fn(),
  searchMessages: vi.fn(),
  getSupportedHashAlgorithms: vi.fn(),
}));

vi.mock('react-toastify', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

describe('MediaCore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mediacore.getContentIdStats.mockResolvedValue({
      mappingsByDomain: {},
      totalDomains: 0,
      totalMappings: 0,
    });
    mediacore.getSupportedHashAlgorithms.mockResolvedValue({
      algorithms: [],
      descriptions: {},
    });
    mediacore.getConflictStrategies.mockResolvedValue([]);
    mediacore.getChannels.mockResolvedValue([]);
    mediacore.getContentOpinions.mockResolvedValue([]);
    mediacore.searchContent.mockResolvedValue([]);
    mediacore.searchMessages.mockResolvedValue([]);
  });

  it('renders a pod workflow index with safety framing', async () => {
    render(<MediaCore />);

    expect(await screen.findByText('Pod Workflow Index')).toBeInTheDocument();
    expect(screen.getByText(/Pod workflows mix read-only diagnostics/)).toBeInTheDocument();
    expect(screen.getByText('Workflow focus')).toBeInTheDocument();
    expect(screen.getAllByText('Show all pod workflows').length).toBeGreaterThan(0);
    expect(screen.getAllByText('DHT Publishing').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Verification').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Signing').length).toBeGreaterThan(0);
    expect(screen.getByText('Publishes metadata')).toBeInTheDocument();
    expect(screen.getAllByText('Handles key material').length).toBeGreaterThan(0);
    expect(screen.getByText('Read-only verification')).toBeInTheDocument();
    expect(screen.getByText(/In Enforce mode, signatures use/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/unique-request-nonce/)).toBeInTheDocument();
    expect(screen.getByText('Publishes pod metadata')).toBeInTheDocument();
    expect(screen.getByText('Mutates local message storage')).toBeInTheDocument();
    expect(screen.getAllByText('Publishes opinion data').length).toBeGreaterThan(0);
    expect(screen.getByRole('link', { name: /DHT Publishing/ })).toHaveAttribute(
      'href',
      '#podcore-dht-publishing',
    );
  });

  it('focuses a pod workflow from the index card', async () => {
    render(<MediaCore />);

    fireEvent.click(await screen.findByRole('link', { name: /DHT Publishing/ }));

    expect(
      screen.getByText(/Showing DHT Publishing/),
    ).toBeInTheDocument();

    fireEvent.click(screen.getAllByText('Show all pod workflows').at(-1));

    expect(screen.queryByText(/Showing DHT Publishing/)).not.toBeInTheDocument();
  });

  it('renders when the compatibility API does not provide algorithm metadata', async () => {
    mediacore.getSupportedHashAlgorithms.mockResolvedValue({
      family: 'mediacore',
      items: [],
      status: 'empty',
      supported: true,
    });

    render(<MediaCore />);

    expect(await screen.findByText('MediaCore ContentID Registry')).toBeInTheDocument();
    expect(screen.getByText('Supported Hash Algorithms')).toBeInTheDocument();
  });

  it('fills every relevant ContentID example field', async () => {
    render(<MediaCore />);

    fireEvent.click(await screen.findByText('audio:track'));

    expect(
      screen.getByPlaceholderText('e.g., mb:recording:12345-6789-...'),
    ).toHaveValue('mb:recording:12345');
    expect(
      screen.getByPlaceholderText('e.g., content:mb:recording:12345-6789-...'),
    ).toHaveValue('content:audio:track:mb-12345');
    expect(
      screen.getByPlaceholderText('Enter external ID to resolve...'),
    ).toHaveValue('mb:recording:12345');
    expect(
      screen
        .getAllByPlaceholderText('e.g., content:audio:track:mb-12345')
        .map((input) => input.value),
    ).toContain('content:audio:track:mb-12345');
  });

  it('reports message-search failures instead of showing no matches', async () => {
    mediacore.searchMessages.mockRejectedValueOnce(
      new Error('Message search unavailable'),
    );

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    fireEvent.change(screen.getByPlaceholderText('Search messages...'), {
      target: { value: 'hello' },
    });
    fireEvent.click(screen.getAllByRole('button', { name: 'Search' })[0]);

    expect(await screen.findByTestId('message-search-error')).toHaveTextContent(
      'Message search unavailable',
    );
    expect(screen.queryByText(/No messages found matching/)).not.toBeInTheDocument();
  });

  it('reports content-search failures explicitly', async () => {
    mediacore.searchContent.mockRejectedValueOnce(
      new Error('Content search unavailable'),
    );

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    fireEvent.change(
      screen.getByPlaceholderText('Search for content (artist, album, movie, etc.)'),
      { target: { value: 'album' } },
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'Search' })[1]);

    expect(await screen.findByTestId('content-search-error')).toHaveTextContent(
      'Content search unavailable',
    );
  });

  it('retains message search results when a same-query refresh fails', async () => {
    mediacore.searchMessages
      .mockResolvedValueOnce([
        {
          body: 'retained message',
          channelId: 'channel-1',
          senderPeerId: 'peer-1',
          timestampUnixMs: 1_700_000_000_000,
        },
      ])
      .mockRejectedValueOnce(new Error('Message search refresh unavailable'));

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    const searchInput = screen.getByPlaceholderText('Search messages...');
    fireEvent.change(searchInput, { target: { value: 'hello' } });
    fireEvent.click(screen.getAllByRole('button', { name: 'Search' })[0]);
    expect(await screen.findByText('retained message')).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole('button', { name: 'Search' })[0]);

    expect(
      await screen.findByTestId('message-search-error'),
    ).toHaveTextContent('Message search refresh unavailable');
    expect(screen.getByText('retained message')).toBeInTheDocument();
    expect(
      screen.getByText('Showing last successfully loaded results.'),
    ).toBeInTheDocument();
  });

  it('retains channels and reports a failed channel refresh', async () => {
    mediacore.getChannels
      .mockResolvedValueOnce([
        { channelId: 'channel-1', kind: 'General', name: 'General' },
      ])
      .mockRejectedValueOnce(new Error('Channel refresh unavailable'));

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    fireEvent.change(screen.getByPlaceholderText('Pod ID for channel management'), {
      target: { value: 'pod-1' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Load Channels' }));
    expect(await screen.findByText(/ID: channel-1/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Load Channels' }));

    expect(
      await screen.findByTestId('media-core-channels-error'),
    ).toHaveTextContent('Channel refresh unavailable');
    expect(screen.getByText(/ID: channel-1/)).toBeInTheDocument();
    expect(
      screen.getByText('Showing last successfully loaded channels.'),
    ).toBeInTheDocument();
  });

  it('retains opinions and reports a failed opinion refresh', async () => {
    mediacore.getContentOpinions
      .mockResolvedValueOnce([
        {
          note: 'trusted source',
          score: 9,
          senderPeerId: 'peer-1',
          variantHash: 'variant-123456',
        },
      ])
      .mockRejectedValueOnce(new Error('Opinion refresh unavailable'));

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    fireEvent.change(screen.getByPlaceholderText('Pod ID'), {
      target: { value: 'pod-1' },
    });
    fireEvent.change(
      screen.getByPlaceholderText('Content ID (e.g., content:audio:album:mb-id)'),
      { target: { value: 'content-1' } },
    );
    fireEvent.click(screen.getByRole('button', { name: 'Get Opinions' }));
    expect(await screen.findByText(/variant-\.\.\./)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Get Opinions' }));

    expect(
      await screen.findByTestId('media-core-opinions-error'),
    ).toHaveTextContent('Opinion refresh unavailable');
    expect(screen.getByText(/variant-\.\.\./)).toBeInTheDocument();
    expect(
      screen.getByText('Showing last successfully loaded opinions.'),
    ).toBeInTheDocument();
  });
});
