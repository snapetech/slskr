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
  getAggregatedOpinions: vi.fn(),
  getBackfillStats: vi.fn(),
  getContentIdStats: vi.fn(),
  getContentOpinions: vi.fn(),
  getChannels: vi.fn(),
  getConsensusRecommendations: vi.fn(),
  getMemberAffinities: vi.fn(),
  getOpinionStatistics: vi.fn(),
  getLastSeenTimestamps: vi.fn(),
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
    mediacore.getAggregatedOpinions.mockResolvedValue({
      consensusStrength: 0,
      contributingMembers: 0,
      totalOpinions: 0,
      uniqueVariants: 0,
      unweightedAverageScore: 0,
      variantAggregates: [],
      weightedAverageScore: 0,
    });
    mediacore.getBackfillStats.mockResolvedValue({
      averageBackfillDurationMs: 0,
      totalBackfillBytesTransferred: 0,
      totalBackfillRequestsReceived: 0,
      totalBackfillRequestsSent: 0,
      totalMessagesBackfilled: 0,
    });
    mediacore.getChannels.mockResolvedValue([]);
    mediacore.getContentOpinions.mockResolvedValue([]);
    mediacore.getConsensusRecommendations.mockResolvedValue([]);
    mediacore.getMemberAffinities.mockResolvedValue({});
    mediacore.getOpinionStatistics.mockResolvedValue({
      averageScore: 0,
      lastUpdated: '2026-09-05T00:00:00.000Z',
      maxScore: 0,
      minScore: 0,
      totalOpinions: 0,
      uniqueVariants: 0,
    });
    mediacore.getLastSeenTimestamps.mockResolvedValue({});
    mediacore.searchContent.mockResolvedValue([]);
    mediacore.searchMessages.mockResolvedValue([]);
  });

  const renderOpinionQuery = async () => {
    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');
    fireEvent.change(screen.getByPlaceholderText('Pod ID'), {
      target: { value: 'pod-1' },
    });
    fireEvent.change(
      screen.getByPlaceholderText('Content ID (e.g., content:audio:album:mb-id)'),
      { target: { value: 'content-1' } },
    );
  };

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

  it('keeps backfill data visible when refreshes fail', async () => {
    mediacore.getBackfillStats
      .mockResolvedValueOnce({
        averageBackfillDurationMs: 12.5,
        totalBackfillBytesTransferred: 1_048_576,
        totalBackfillRequestsReceived: 2,
        totalBackfillRequestsSent: 3,
        totalMessagesBackfilled: 4,
      })
      .mockRejectedValueOnce(new Error('Backfill statistics unavailable'));
    mediacore.getLastSeenTimestamps
      .mockResolvedValueOnce({
        'channel-1': '2026-09-05T00:00:00.000Z',
      })
      .mockRejectedValueOnce(new Error('Last-seen timestamps unavailable'));

    render(<MediaCore />);
    await screen.findByText('MediaCore ContentID Registry');

    fireEvent.click(screen.getByRole('button', { name: 'Get Backfill Stats' }));
    expect((await screen.findByText(/Requests Sent:/)).parentElement).toHaveTextContent(
      '3',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Get Backfill Stats' }));
    expect(
      await screen.findByTestId('media-core-backfill-stats-error'),
    ).toHaveTextContent('Backfill statistics unavailable');
    expect(screen.getByText(/Requests Sent:/).parentElement).toHaveTextContent('3');

    const podInput = screen.getByPlaceholderText('Pod ID for backfill sync');
    fireEvent.change(podInput, { target: { value: 'pod-1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Get Timestamps' }));
    expect(
      await screen.findByText('Last Seen Timestamps for Pod pod-1'),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Get Timestamps' }));
    expect(
      await screen.findByTestId('media-core-last-seen-error'),
    ).toHaveTextContent('Last-seen timestamps unavailable');
    expect(screen.getByText(/channel-1:/)).toBeInTheDocument();
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

  it('keeps opinion statistics visible when their refresh fails', async () => {
    mediacore.getOpinionStatistics
      .mockResolvedValueOnce({
        averageScore: 8,
        lastUpdated: '2026-09-05T00:00:00.000Z',
        maxScore: 10,
        minScore: 6,
        totalOpinions: 2,
        uniqueVariants: 1,
      })
      .mockRejectedValueOnce(new Error('Statistics refresh unavailable'));

    await renderOpinionQuery();

    fireEvent.click(screen.getByRole('button', { name: 'Get Statistics' }));
    expect((await screen.findByText(/Average Score:/)).parentElement).toHaveTextContent(
      '8.0',
    );
    fireEvent.click(screen.getByRole('button', { name: 'Get Statistics' }));
    expect(
      await screen.findByTestId('media-core-opinion-statistics-error'),
    ).toHaveTextContent('Statistics refresh unavailable');
    expect(screen.getByText(/Average Score:/).parentElement).toHaveTextContent(
      '8.0',
    );
  });

  it('keeps aggregated opinion summaries visible when their refresh fails', async () => {
    mediacore.getAggregatedOpinions
      .mockResolvedValueOnce({
        consensusStrength: 0.8,
        contributingMembers: 2,
        totalOpinions: 2,
        uniqueVariants: 1,
        unweightedAverageScore: 8,
        variantAggregates: [],
        weightedAverageScore: 8.5,
      })
      .mockRejectedValueOnce(new Error('Aggregate refresh unavailable'));

    await renderOpinionQuery();

    fireEvent.click(
      screen.getByRole('button', { name: 'Get Aggregated Opinions' }),
    );
    expect((await screen.findByText(/Weighted Average:/)).parentElement).toHaveTextContent(
      '8.50',
    );
    fireEvent.click(
      screen.getByRole('button', { name: 'Get Aggregated Opinions' }),
    );
    expect(
      await screen.findByTestId('media-core-aggregated-opinions-error'),
    ).toHaveTextContent('Aggregate refresh unavailable');
    expect(screen.getByText(/Weighted Average:/).parentElement).toHaveTextContent(
      '8.50',
    );
  });

  it('keeps member affinity summaries visible when their refresh fails', async () => {
    mediacore.getMemberAffinities
      .mockResolvedValueOnce({
        'peer-12345678': {
          affinityScore: 0.8,
          lastActivity: '2026-09-05T00:00:00.000Z',
          messageCount: 4,
          opinionCount: 2,
          trustScore: 0.9,
        },
      })
      .mockRejectedValueOnce(new Error('Affinity refresh unavailable'));

    await renderOpinionQuery();

    fireEvent.click(
      screen.getByRole('button', { name: 'Get Member Affinities' }),
    );
    expect(await screen.findByText('Member Affinities (1)')).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole('button', { name: 'Get Member Affinities' }),
    );
    expect(
      await screen.findByTestId('media-core-member-affinities-error'),
    ).toHaveTextContent('Affinity refresh unavailable');
    expect(screen.getByText('Member Affinities (1)')).toBeInTheDocument();
  });

  it('keeps consensus recommendations visible when their refresh fails', async () => {
    mediacore.getConsensusRecommendations
      .mockResolvedValueOnce([
        {
          consensusScore: 0.8,
          reasoning: 'consistent scores',
          recommendation: 'Recommended',
          supportingFactors: ['agreement'],
          variantHash: 'variant-123456',
        },
      ])
      .mockRejectedValueOnce(new Error('Recommendation refresh unavailable'));

    await renderOpinionQuery();

    fireEvent.click(screen.getByRole('button', { name: 'Get Recommendations' }));
    expect(
      (await screen.findByText(/Recommendation:/)).parentElement,
    ).toHaveTextContent('Recommended');
    fireEvent.click(screen.getByRole('button', { name: 'Get Recommendations' }));
    expect(
      await screen.findByTestId('media-core-consensus-recommendations-error'),
    ).toHaveTextContent('Recommendation refresh unavailable');
    expect(screen.getByText(/Recommendation:/).parentElement).toHaveTextContent(
      'Recommended',
    );
  });
});
