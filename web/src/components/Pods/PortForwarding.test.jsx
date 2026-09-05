import '@testing-library/jest-dom';
import * as pods from '../../lib/pods';
import * as portForwarding from '../../lib/portForwarding';
import PortForwarding from './PortForwarding';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/pods', () => ({
  get: vi.fn(),
  list: vi.fn(),
}));

vi.mock('../../lib/portForwarding', () => ({
  getAvailablePorts: vi.fn(),
  getForwardingStatus: vi.fn(),
  startForwarding: vi.fn(),
  stopForwarding: vi.fn(),
}));

vi.mock('../../lib/usePolling', () => ({
  createPollingController: vi.fn(() => ({ stop: vi.fn() })),
}));

describe('PortForwarding', () => {
  beforeEach(() => {
    pods.list.mockRejectedValue(new Error('Pod service unavailable'));
    portForwarding.getAvailablePorts.mockResolvedValue({ availablePorts: [] });
    portForwarding.getForwardingStatus.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('reports pod-list failures instead of claiming no VPN-capable pods exist', async () => {
    render(<PortForwarding />);

    expect(await screen.findByTestId('port-forwarding-pods-error')).toHaveTextContent(
      'Pod service unavailable',
    );
    expect(screen.queryByText('No VPN-Capable Pods')).not.toBeInTheDocument();
  });
});
