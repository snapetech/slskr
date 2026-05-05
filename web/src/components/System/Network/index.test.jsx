// <copyright file="index.test.jsx" company="slskr Team">
// Copyright (c) slskr Team. All rights reserved.
// </copyright>

import * as slskrAPI from '../../../lib/slskr';
import Network from '.';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';

vi.mock('../../../lib/slskr');
vi.mock('../../Shared', () => ({
  LoaderSegment: () => <div>Loading...</div>,
  ShrinkableButton: ({ children, ...props }) => (
    <button {...props}>{children}</button>
  ),
}));
vi.mock('react-toastify', () => ({
  toast: {
    error: vi.fn(),
    info: vi.fn(),
    success: vi.fn(),
  },
}));

describe('Network', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
    window.localStorage.clear();
    slskrAPI.getSlskrStats.mockResolvedValue({
      backfill: {
        completedToday: 0,
        discoveryRate: 0,
        isActive: false,
        pendingCount: 0,
      },
      capabilities: { features: [], version: 'slskr' },
      dht: {
        dhtNodeCount: 0,
        isEnabled: true,
        isLanOnly: false,
        isDhtRunning: true,
      },
      hashDb: { currentSeqId: 0, totalEntries: 0 },
      mesh: {
        connectedPeerCount: 0,
        warnings: [],
      },
      swarmJobs: [],
    });
    slskrAPI.getMeshPeers.mockResolvedValue([]);
    slskrAPI.getDiscoveredPeers.mockResolvedValue([]);
  });

  it('shows the connectivity diagnostics warning when no peers are reachable', async () => {
    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.getByText('Connectivity diagnostics')).toBeInTheDocument();
    });

    expect(
      screen.getByText(/configured Soulseek listen port is reachable/i),
    ).toBeInTheDocument();
    expect(screen.getByText('Network Health')).toBeInTheDocument();
    expect(screen.getByText('Needs attention')).toBeInTheDocument();
  });

  it('copies a network health report', async () => {
    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.getByText('Network Health')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: 'Copy network health report' }));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        expect.stringContaining('slskr network health report'),
      );
    });
  });

  it('explains zero-node DHT when LAN-only mode disables public bootstrap', async () => {
    slskrAPI.getSlskrStats.mockResolvedValueOnce({
      backfill: {
        completedToday: 0,
        discoveryRate: 0,
        isActive: false,
        pendingCount: 0,
      },
      capabilities: { features: [], version: 'slskr' },
      dht: {
        dhtNodeCount: 0,
        isEnabled: true,
        isLanOnly: true,
        isDhtRunning: true,
      },
      hashDb: { currentSeqId: 0, totalEntries: 0 },
      mesh: {
        connectedPeerCount: 0,
        warnings: [],
      },
      swarmJobs: [],
    });

    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.getByText('LAN-only DHT is isolated')).toBeInTheDocument();
    });

    expect(screen.queryByText('Connectivity diagnostics')).not.toBeInTheDocument();
    expect(
      screen.getByText(/intentionally skips the public BitTorrent DHT bootstrap/i),
    ).toBeInTheDocument();
  });

  it('shows a dismissable DHT exposure notice for first-run public DHT usage', async () => {
    const { container } = render(<Network theme="light" />);

    await waitFor(() => {
      expect(
        screen.getByText('Public DHT exposure notice'),
      ).toBeInTheDocument();
    });

    fireEvent.click(container.querySelector('.close.icon'));

    await waitFor(() => {
      expect(
        screen.queryByText('Public DHT exposure notice'),
      ).not.toBeInTheDocument();
    });

    expect(
      window.localStorage.getItem('slskr:ui:dht-public-exposure:consent-v1'),
    ).toBe('acknowledged');
  });

  it('does not show the DHT exposure notice if already acknowledged', async () => {
    window.localStorage.setItem('slskr:ui:dht-public-exposure:consent-v1', 'acknowledged');

    render(<Network theme="light" />);

    await waitFor(() => {
      expect(
        screen.queryByText('Public DHT exposure notice'),
      ).not.toBeInTheDocument();
    });
  });

  it('does not show connectivity diagnostics when DHT status has peers', async () => {
    window.localStorage.setItem('slskr:ui:dht-public-exposure:consent-v1', 'acknowledged');
    slskrAPI.getSlskrStats.mockResolvedValueOnce({
      backfill: {
        completedToday: 0,
        discoveryRate: 0,
        isActive: false,
        pendingCount: 0,
      },
      capabilities: { features: [], version: 'slskr' },
      dht: {
        activeMeshConnections: 1,
        dhtNodeCount: 155,
        discoveredPeerCount: 37,
        isEnabled: true,
        isLanOnly: false,
        isDhtRunning: true,
      },
      hashDb: { currentSeqId: 0, totalEntries: 0 },
      mesh: {
        connectedPeerCount: 0,
        warnings: [],
      },
      swarmJobs: [],
    });

    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.getByText('Mesh Sync Security')).toBeInTheDocument();
    });

    expect(screen.queryByText('Connectivity diagnostics')).not.toBeInTheDocument();
    expect(screen.getByText('Healthy')).toBeInTheDocument();
  });

  it('does not show the DHT exposure notice when DHT is LAN-only', async () => {
    slskrAPI.getSlskrStats.mockResolvedValueOnce({
      backfill: {
        completedToday: 0,
        discoveryRate: 0,
        isActive: false,
        pendingCount: 0,
      },
      capabilities: { features: [], version: 'slskr' },
      dht: {
        dhtNodeCount: 3,
        isEnabled: true,
        isLanOnly: true,
        isDhtRunning: true,
      },
      hashDb: { currentSeqId: 0, totalEntries: 0 },
      mesh: {
        connectedPeerCount: 0,
        warnings: [],
      },
      swarmJobs: [],
    });

    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.queryByText('Public DHT exposure notice')).not.toBeInTheDocument();
    });
  });

  it('does not show the DHT exposure notice when the backend reports lanOnly', async () => {
    slskrAPI.getSlskrStats.mockResolvedValueOnce({
      backfill: {
        completedToday: 0,
        discoveryRate: 0,
        isActive: false,
        pendingCount: 0,
      },
      capabilities: { features: [], version: 'slskr' },
      dht: {
        dhtNodeCount: 3,
        isEnabled: true,
        lanOnly: true,
        isDhtRunning: true,
      },
      hashDb: { currentSeqId: 0, totalEntries: 0 },
      mesh: {
        connectedPeerCount: 0,
        warnings: [],
      },
      swarmJobs: [],
    });

    render(<Network theme="light" />);

    await waitFor(() => {
      expect(screen.queryByText('Public DHT exposure notice')).not.toBeInTheDocument();
    });
  });

  it('renders inverted statistics in dark theme', async () => {
    const { container } = render(<Network theme="dark" />);

    await waitFor(() => {
      expect(screen.getByText('Mesh Sync Security')).toBeInTheDocument();
    });

    expect(container.querySelector('.ui.inverted.statistics')).not.toBeNull();
  });
});
