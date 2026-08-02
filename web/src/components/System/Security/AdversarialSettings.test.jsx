import '@testing-library/jest-dom';
import * as securityApi from '../../../lib/security';
import AdversarialSettings from './AdversarialSettings';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/security', () => ({
  getAdversarialSettings: vi.fn(),
  getAdversarialStats: vi.fn(),
  getTorStatus: vi.fn(),
  getTransportStatus: vi.fn(),
  updateAdversarialSettings: vi.fn(),
}));

describe('Adversarial Settings probes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    securityApi.getAdversarialStats.mockResolvedValue({});
    securityApi.getTorStatus.mockResolvedValue({});
    securityApi.getTransportStatus.mockResolvedValue({});
  });

  it('does not probe optional transports when adversarial features are disabled', async () => {
    securityApi.getAdversarialSettings.mockResolvedValue({
      enabled: false,
      anonymity: { enabled: false, mode: 'Direct' },
    });

    render(<AdversarialSettings />);

    await waitFor(() => expect(securityApi.getAdversarialStats).toHaveBeenCalledTimes(1));
    expect(securityApi.getTransportStatus).not.toHaveBeenCalled();
    expect(securityApi.getTorStatus).not.toHaveBeenCalled();
    expect(screen.getByText('Adversarial Resilience Overview')).toBeInTheDocument();
  });

  it('probes transport and Tor only when configured and enabled', async () => {
    securityApi.getAdversarialSettings.mockResolvedValue({
      Enabled: true,
      Anonymity: { Enabled: true, Mode: 'Tor' },
    });

    render(<AdversarialSettings />);

    await waitFor(() => expect(securityApi.getTransportStatus).toHaveBeenCalledTimes(1));
    expect(securityApi.getTorStatus).toHaveBeenCalledTimes(1);
  });
});
