import '@testing-library/jest-dom';
import * as pods from '../../lib/pods';
import Pods from './Pods';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/pods', () => ({
  create: vi.fn(),
  discoverAll: vi.fn(),
  discoverByName: vi.fn(),
  get: vi.fn(),
  getMembers: vi.fn(),
  getMessages: vi.fn(),
  join: vi.fn(),
  leave: vi.fn(),
  list: vi.fn(),
  sendMessage: vi.fn(),
}));

vi.mock('../../lib/usePolling', () => ({
  createPollingController: vi.fn(() => ({ stop: vi.fn() })),
}));

vi.mock('./PortForwarding', () => ({ default: () => null }));
vi.mock('./VpnGatewayConfig', () => ({ default: () => null }));
vi.mock('../Player/PodListenAlongPanel', () => ({ default: () => null }));

describe('Pods', () => {
  beforeEach(() => {
    pods.list.mockRejectedValue(new Error('Pod service unavailable'));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('reports pod-list failures instead of showing an empty success state', async () => {
    render(
      <MemoryRouter>
        <Pods />
      </MemoryRouter>,
    );

    expect(await screen.findByTestId('pods-load-error')).toHaveTextContent(
      'Pod service unavailable',
    );
    expect(screen.getByTestId('pods-load-error')).toHaveTextContent(
      'Pods unavailable',
    );
    expect(screen.queryByText('No pods yet')).not.toBeInTheDocument();
  });
});
