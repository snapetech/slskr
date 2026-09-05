import '@testing-library/jest-dom';
import * as pods from '../../lib/pods';
import Pods from './Pods';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
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

  it('reports pod-detail failures instead of showing an unselected state', async () => {
    pods.list.mockResolvedValue([
      {
        channels: [{ channelId: 'general', kind: 'General', name: 'General' }],
        name: 'Test pod',
        podId: 'pod-1',
      },
    ]);
    pods.get.mockRejectedValue(new Error('Pod detail service unavailable'));
    pods.getMembers.mockResolvedValue([]);

    render(
      <MemoryRouter>
        <Pods />
      </MemoryRouter>,
    );

    expect(await screen.findByTestId('pod-detail-load-error')).toHaveTextContent(
      'Pod detail service unavailable',
    );
    expect(screen.getByText('Pod details unavailable')).toBeInTheDocument();
    expect(screen.queryByText('Select a pod to view details')).not.toBeInTheDocument();
  });

  it('reports message-history failures instead of showing an empty success state', async () => {
    pods.list.mockResolvedValue([
      {
        channels: [{ channelId: 'general', kind: 'General', name: 'General' }],
        name: 'Test pod',
        podId: 'pod-1',
      },
    ]);
    pods.get.mockResolvedValue({
      channels: [{ channelId: 'general', kind: 'General', name: 'General' }],
      name: 'Test pod',
      podId: 'pod-1',
    });
    pods.getMembers.mockResolvedValue([]);
    pods.getMessages.mockRejectedValue(new Error('Message service unavailable'));

    render(
      <MemoryRouter>
        <Pods />
      </MemoryRouter>,
    );

    expect(await screen.findByTestId('pod-messages-load-error')).toHaveTextContent(
      'Message service unavailable',
    );
    expect(screen.getByText('Messages unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No messages yet')).not.toBeInTheDocument();
  });

  it('reports discovery failures instead of hiding the failed lookup', async () => {
    pods.list.mockResolvedValue([]);
    pods.discoverAll.mockRejectedValue(new Error('Discovery service unavailable'));

    render(
      <MemoryRouter>
        <Pods />
      </MemoryRouter>,
    );

    const searchButton = await screen.findByRole('button', { name: 'Discover pods' });
    fireEvent.click(searchButton);

    expect(await screen.findByTestId('pods-discovery-error')).toHaveTextContent(
      'Discovery service unavailable',
    );
    expect(screen.getByText('Pod discovery unavailable')).toBeInTheDocument();
  });
});
