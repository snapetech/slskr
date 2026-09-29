// <copyright file="index.test.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as mediacore from '../../../lib/mediacore';
import MediaCore from './index';
import React, { act } from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/mediacore', () => ({
  getConflictStrategies: vi.fn(),
  getAggregatedOpinions: vi.fn(),
  getBackfillStats: vi.fn(),
  getContentRegistryStats: vi.fn(),
  getContentIdStats: vi.fn(),
  getContentOpinions: vi.fn(),
  getChannels: vi.fn(),
  getMessageStorageStats: vi.fn(),
  getConsensusRecommendations: vi.fn(),
  getMemberAffinities: vi.fn(),
  getOpinionStatistics: vi.fn(),
  getLastSeenTimestamps: vi.fn(),
  searchContent: vi.fn(),
  searchMessages: vi.fn(),
  getSupportedHashAlgorithms: vi.fn(),
  resolveContentId: vi.fn(),
}));

vi.mock('react-toastify', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

describe('MediaCore', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mediacore.getContentIdStats.mockResolvedValue({
      mappingsByDomain: {},
      totalDomains: 0,
      totalMappings: 0,
    });
    mediacore.getMessageStorageStats.mockResolvedValue({
      messagesPerChannel: {},
      messagesPerPod: {},
      newestMessage: null,
      oldestMessage: null,
      totalMessages: 0,
      totalSizeBytes: 0,
    });
    mediacore.getContentRegistryStats.mockResolvedValue({
      averageMappingsPerDomain: 0,
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
    mediacore.resolveContentId.mockResolvedValue(null);
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

  it('profiles message-search input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const searchInput = screen.getByPlaceholderText('Search messages...');
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(searchInput, { target: { value: `warmup-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(searchInput, { target: { value: `profile-${index}` } });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod publication input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const podInput = screen.getByPlaceholderText(
      '{"id": {"value": "pod:artist:mb:daft-punk-hash"}, "displayName": "Daft Punk Fans", "visibility": "Listed", "focusType": "ContentId", "focusContentId": {"domain": "audio", "type": "artist", "id": "daft-punk-hash"}, "tags": ["electronic", "french-house"], "createdAt": "2024-01-01T00:00:00Z", "createdBy": "alice", "metadata": {"description": "A community for Daft Punk fans", "memberCount": 150}}',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, { target: { value: `warmup-pod-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, { target: { value: `profile-pod-${index}` } });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_POD_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod membership input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const membershipInput = screen.getByPlaceholderText(
      '{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "role": "member", "isBanned": false, "publicKey": "base64-ed25519-key", "joinedAt": "2024-01-01T00:00:00Z"}',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(membershipInput, {
        target: { value: `warmup-membership-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(membershipInput, {
        target: { value: `profile-membership-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_MEMBERSHIP_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod discovery input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const podInput = screen.getByPlaceholderText(
      '{"podId": "pod:artist:mb:daft-punk-hash", "name": "Daft Punk Fans", "visibility": "Listed", "focusContentId": "content:audio:artist:daft-punk", "tags": ["electronic", "french-house"]}',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, {
        target: { value: `warmup-discovery-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, {
        target: { value: `profile-discovery-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_DISCOVERY_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod join request input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const joinInput = screen.getByPlaceholderText(
      '{"podId": "pod:artist:mb:daft-punk-hash", "peerId": "alice", "requestedRole": "member", "publicKey": "base64-ed25519-public-key", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature", "nonce": "unique-request-nonce", "message": "Please let me join!"}',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(joinInput, { target: { value: `warmup-join-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(joinInput, {
        target: { value: `profile-join-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_JOIN_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod message routing input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const routingPanel = document.getElementById('pod-message-routing');
    const messageInput = within(routingPanel).getByPlaceholderText(
      '{"messageId": "msg123", "channelId": "pod:artist:mb:daft-punk-hash:general", "senderPeerId": "alice", "body": "Hello pod!", "timestampUnixMs": 1703123456789, "signature": "ed25519:base64-signature"}',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(messageInput, {
        target: { value: `warmup-message-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(messageInput, {
        target: { value: `profile-message-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_ROUTING_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod message backfill input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const backfillPanel = document.getElementById('pod-message-backfill');
    const podInput = within(backfillPanel).getByPlaceholderText(
      'Pod ID for backfill sync',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, { target: { value: `warmup-backfill-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, {
        target: { value: `profile-backfill-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_BACKFILL_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod channel management input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const channelPanel = document.getElementById('pod-channel-management');
    const podInput = within(channelPanel).getByPlaceholderText(
      'Pod ID for channel management',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, { target: { value: `warmup-channel-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, {
        target: { value: `profile-channel-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_CHANNEL_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod content search input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const contentPanel = document.getElementById('pod-content-linking');
    const queryInput = within(contentPanel).getByPlaceholderText(
      'Search for content (artist, album, movie, etc.)',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(queryInput, { target: { value: `warmup-content-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(queryInput, {
        target: { value: `profile-content-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_CONTENT_LINKING_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod opinion input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const opinionPanel = document.getElementById('pod-opinion-management');
    const podInput = within(opinionPanel).getByPlaceholderText('Pod ID');
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, { target: { value: `warmup-opinion-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, {
        target: { value: `profile-opinion-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_OPINION_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod message signing input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const signingPanel = document.getElementById('pod-message-signing');
    const messageInput = within(signingPanel).getAllByRole('textbox')[0];
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(messageInput, {
        target: { value: `warmup-signing-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(messageInput, {
        target: { value: `profile-signing-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    console.log('MEDIA_CORE_SIGNING_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: sorted[Math.floor(sorted.length / 2)],
      p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
      samplesMs: inputSamples,
    }));
  });

  it('profiles content-ID registry input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const resolveInput = screen.getByPlaceholderText(
      'Enter external ID to resolve...',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(resolveInput, {
        target: { value: `warmup-resolve-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(resolveInput, {
        target: { value: `profile-resolve-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_REGISTRY_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles MediaCore dashboard statistics updates', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const loadRegistryStats = screen.getByRole('button', {
      name: 'Load Registry Stats',
    });
    for (let index = 0; index < 3; index += 1) {
      fireEvent.click(loadRegistryStats);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.click(loadRegistryStats);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_DASHBOARD_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles descriptor publishing input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const publishCard = screen.getByText('Publish Content Descriptor').closest('.ui.card');
    const contentInput = within(publishCard).getByPlaceholderText(
      'content:audio:track:mb-12345',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(contentInput, {
        target: { value: `warmup-descriptor-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(contentInput, {
        target: { value: `profile-descriptor-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_DESCRIPTOR_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles ContentID registry input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const registerCard = screen.getByText('Register ContentID Mapping').closest('.ui.card');
    const externalIdInput = within(registerCard).getByPlaceholderText(
      'e.g., mb:recording:12345-6789-...',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(externalIdInput, {
        target: { value: `warmup-registry-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(externalIdInput, {
        target: { value: `profile-registry-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_CONTENT_REGISTRY_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles content graph traversal input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const graphCard = screen.getByText('IPLD Graph Traversal').closest('.ui.card');
    const traversalInput = within(graphCard).getAllByRole('textbox')[0];
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(traversalInput, {
        target: { value: `warmup-graph-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(traversalInput, {
        target: { value: `profile-graph-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_GRAPH_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles metadata import input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const importInput = screen.getByPlaceholderText(
      'Paste exported metadata package JSON here...',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(importInput, {
        target: { value: `warmup-metadata-${index}` },
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(importInput, {
        target: { value: `profile-metadata-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_METADATA_IMPORT_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod membership verification input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const verificationPanel = document.getElementById(
      'pod-membership-verification',
    );
    const podInput = within(verificationPanel).getAllByPlaceholderText(
      'pod:artist:mb:daft-punk-hash',
    )[0];
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(podInput, { target: { value: `warmup-verify-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(podInput, {
        target: { value: `profile-verify-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_MEMBERSHIP_VERIFICATION_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles pod message storage updates', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const storageButton = screen.getByRole('button', { name: 'Get Storage Stats' });
    for (let index = 0; index < 3; index += 1) {
      fireEvent.click(storageButton);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.click(storageButton);
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_MESSAGE_STORAGE_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles audio hash input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const audioInput = screen.getByPlaceholderText(
      '0.1, -0.2, 0.3, ... (normalized -1.0 to 1.0)',
    );
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(audioInput, { target: { value: `warmup-audio-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(audioInput, {
        target: { value: `profile-audio-${index}` },
      });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_AUDIO_HASH_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('profiles hash similarity input render commits', async () => {
    const durations = [];
    const onRender = (_id, phase, actualDuration) => {
      if (phase === 'update') durations.push(actualDuration);
    };

    render(
      <React.Profiler id="MediaCore" onRender={onRender}>
        <MediaCore />
      </React.Profiler>,
    );
    await screen.findByText('MediaCore ContentID Registry');
    const hashInput = screen.getByPlaceholderText('First hash value (hexadecimal)');
    for (let index = 0; index < 3; index += 1) {
      fireEvent.change(hashInput, { target: { value: `warmup-hash-${index}` } });
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 50));
    });

    durations.length = 0;
    const inputSamples = [];
    for (let index = 0; index < 20; index += 1) {
      const previousCommitCount = durations.length;
      fireEvent.change(hashInput, { target: { value: `profile-hash-${index}` } });
      expect(durations.length).toBeGreaterThan(previousCommitCount);
      inputSamples.push(durations[previousCommitCount]);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }

    expect(inputSamples).toHaveLength(20);
    expect(inputSamples.every(Number.isFinite)).toBe(true);
    const sorted = [...inputSamples].sort((left, right) => left - right);
    const median = sorted[Math.floor(sorted.length / 2)];
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    console.log('MEDIA_CORE_HASH_SIMILARITY_PROFILE', JSON.stringify({
      commits: inputSamples.length,
      medianMs: median,
      p95Ms: p95,
      samplesMs: inputSamples,
    }));
  });

  it('resolves an external ID from the content registry', async () => {
    mediacore.resolveContentId.mockResolvedValue({
      contentId: 'content:audio:track:mb-123',
      externalId: 'mb:recording:123',
    });
    render(<MediaCore />);

    await screen.findByText('MediaCore ContentID Registry');
    fireEvent.change(
      screen.getByPlaceholderText('Enter external ID to resolve...'),
      { target: { value: ' mb:recording:123 ' } },
    );
    fireEvent.click(screen.getByRole('button', { name: 'Resolve' }));

    expect(await screen.findByText('Resolved Successfully')).toBeInTheDocument();
    expect(mediacore.resolveContentId).toHaveBeenCalledWith('mb:recording:123');
    expect(
      screen.getByText(/content:audio:track:mb-123/),
    ).toBeInTheDocument();
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

    fireEvent.change(await screen.findByPlaceholderText('Pod ID'), {
      target: { value: 'pod-1' },
    });
    fireEvent.change(
      await screen.findByPlaceholderText(
        'Content ID (e.g., content:audio:album:mb-id)',
      ),
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
