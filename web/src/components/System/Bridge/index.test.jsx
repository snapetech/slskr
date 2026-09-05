import '@testing-library/jest-dom';
import * as bridge from '../../../lib/bridge';
import { usePolling } from '../../../lib/usePolling';
import Bridge from './index';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/bridge', () => ({
  getConfig: vi.fn(),
  getDashboard: vi.fn(),
  startBridge: vi.fn(),
  stopBridge: vi.fn(),
  updateConfig: vi.fn(),
}));

vi.mock('../../../lib/usePolling', () => ({
  usePolling: vi.fn(),
}));

describe('Bridge', () => {
  let poll;

  beforeEach(() => {
    poll = undefined;
    usePolling.mockImplementation((callback) => {
      poll = callback;
    });
    bridge.getConfig.mockResolvedValue({ enabled: true });
    bridge.getDashboard.mockResolvedValue({
      health: { isHealthy: true, version: '1.0.0' },
      stats: { currentConnections: 12 },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('reports polling failures while retaining the last dashboard snapshot', async () => {
    render(<Bridge />);

    await waitFor(() => {
      expect(screen.getByText('12')).toBeInTheDocument();
    });

    bridge.getDashboard.mockRejectedValueOnce({
      response: { data: { message: 'Bridge telemetry unavailable' } },
    });

    await act(async () => {
      await poll();
    });

    expect(screen.getByTestId('bridge-dashboard-error')).toHaveTextContent(
      'Bridge telemetry unavailable',
    );
    expect(screen.getByText('12')).toBeInTheDocument();
  });

  it('preserves target-compatible connected client counts without fabricating client rows', async () => {
    bridge.getDashboard.mockResolvedValue({
      connectedClients: 4,
      health: 'Healthy',
      meshBenefits: { enabled: true },
      stats: { currentConnections: 4 },
    });

    render(<Bridge />);

    expect(await screen.findByText('Connected Clients (4)')).toBeInTheDocument();
    expect(screen.getByTestId('bridge-client-details-unavailable')).toHaveTextContent(
      '4 client(s) are connected',
    );
    expect(screen.queryByText('No clients connected')).not.toBeInTheDocument();
    expect(screen.queryByText('Mesh Benefits')).not.toBeInTheDocument();
  });
});
