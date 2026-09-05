import '@testing-library/jest-dom';
import Footer from './Footer';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';

const { getBuild, getSlskrStats, getStats, getSpeeds, isLoggedIn } = vi.hoisted(() => ({
  getBuild: vi.fn(),
  getSlskrStats: vi.fn(),
  getSpeeds: vi.fn(),
  getStats: vi.fn(),
  isLoggedIn: vi.fn(),
}));

vi.mock('../../lib/application', () => ({
  getBuild,
}));

vi.mock('../../lib/mesh', () => ({
  getStats,
}));

vi.mock('../../lib/session', () => ({
  isLoggedIn,
}));

vi.mock('../../lib/slskr', () => ({
  getSlskrStats,
}));

vi.mock('../../lib/transfers', () => ({
  getSpeeds,
}));

describe('Footer', () => {
  beforeEach(() => {
    isLoggedIn.mockReturnValue(true);
    getStats.mockResolvedValue({
      dht: 12,
      natType: 'FullCone',
      overlay: 3,
    });
    getSlskrStats.mockResolvedValue({
      backfill: { isActive: true },
      dht: { discoveredPeerCount: 23, dhtNodeCount: 12 },
      hashDb: { currentSeqId: 14638, totalEntries: 7_300 },
      mesh: { connectedPeerCount: 6, isSyncing: true },
      swarmJobs: [{ id: 'job-1' }],
    });
    getSpeeds.mockResolvedValue({
      mesh: 2_048,
      soulseek: 4_096,
      total: 6_144,
    });
    getBuild.mockResolvedValue({
      current: '0.0.0-slskr.manual.test',
      full: '0.0.0-slskr.manual.test (0.0.0-slskr.manual.test)',
      isUpdateAvailable: false,
      latest: '0.0.0-slskr.manual.test',
      latestTag: '0.0.0-slskr.manual.test',
      latestUrl: 'https://github.com/snapetech/slskr/releases/tag/test',
    });
    localStorage.setItem('slskr-karma', '2');
  });

  afterEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('renders slskr network stats in the footer', async () => {
    render(<Footer />);

    await waitFor(() => {
      expect(screen.getByText('23 dht')).toBeInTheDocument();
    });

    expect(screen.getByText('6 mesh')).toBeInTheDocument();
    expect(screen.getByText('7.3K hashes')).toBeInTheDocument();
    expect(screen.getByText('seq:14638')).toBeInTheDocument();
    expect(screen.getByText('1 swarm')).toBeInTheDocument();
    expect(screen.getByText('backfill')).toBeInTheDocument();
    expect(screen.getByText('+2')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /paypal/i })).toHaveAttribute(
      'href',
      'https://www.paypal.com/donate/?business=donations%40snape.tech',
    );
    expect(screen.getByRole('link', { name: /ko-fi/i })).toHaveAttribute(
      'href',
      'https://ko-fi.com/snapetech',
    );
  });

  it('shows only attribution when logged out — no build badge, donation links, or telemetry', async () => {
    isLoggedIn.mockReturnValue(false);
    getBuild.mockResolvedValue({
      current: '0.0.0-slskr.manual.local',
      full: '0.0.0-slskr.manual.local (0.0.0-slskr.manual.local)',
      isUpdateAvailable: true,
      latest: '2026050500-slskr.221',
      latestTag: 'build-main-2026050500-slskr.221',
      latestUrl: 'https://github.com/snapetech/slskr/releases/tag/build-main-2026050500-slskr.221',
    });

    render(<Footer />);

    expect(await screen.findByText('slskr')).toBeInTheDocument();
    expect(screen.queryByText('0.0.0-slskr.manual.local')).not.toBeInTheDocument();
    expect(screen.queryByText(/PayPal/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Ko-fi/)).not.toBeInTheDocument();
    expect(getStats).not.toHaveBeenCalled();
    expect(getSlskrStats).not.toHaveBeenCalled();
    expect(getSpeeds).not.toHaveBeenCalled();
  });

  it('distinguishes unavailable telemetry from measured zero values', async () => {
    getStats.mockRejectedValue(new Error('Network stats unavailable'));
    getSlskrStats.mockRejectedValue(new Error('slskr stats unavailable'));
    getSpeeds.mockRejectedValue(new Error('Transfer speeds unavailable'));

    render(<Footer />);

    expect(await screen.findByText('— dht')).toBeInTheDocument();
    expect(screen.getByText('— mesh')).toBeInTheDocument();
    expect(screen.getByText('— hashes')).toBeInTheDocument();
    expect(screen.getAllByText('—').length).toBeGreaterThan(0);
  });
});
