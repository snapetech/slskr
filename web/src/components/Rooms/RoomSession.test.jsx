import '@testing-library/jest-dom';
import * as rooms from '../../lib/rooms';
import RoomSession from './RoomSession';
import React from 'react';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { pollers } = vi.hoisted(() => ({ pollers: [] }));

vi.mock('../../lib/rooms', () => ({
  getMessages: vi.fn(),
  getUsers: vi.fn(),
  sendMessage: vi.fn(),
}));

vi.mock('../../lib/hubFactory', () => ({
  createRoomsHubConnection: vi.fn(() => ({
    on: vi.fn(),
    start: vi.fn().mockResolvedValue(undefined),
    stop: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock('../../lib/usePolling', () => ({
  createPollingController: vi.fn((callback) => {
    const controller = {
      refresh: () => callback(),
      stop: vi.fn(),
    };
    pollers.push(controller);
    void callback();
    return controller;
  }),
}));

describe('RoomSession', () => {
  beforeEach(() => {
    pollers.length = 0;
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('requests room deltas and keeps earlier messages', async () => {
    rooms.getMessages
      .mockResolvedValueOnce([
        { createdAtMs: 100, id: 'one', message: 'one', username: 'peer' },
      ])
      .mockResolvedValueOnce([
        { createdAtMs: 200, id: 'two', message: 'two', username: 'peer' },
      ]);
    rooms.getUsers.mockResolvedValue([]);

    render(<RoomSession active roomName="lobby" />);

    expect(await screen.findByText('one')).toBeInTheDocument();
    await act(async () => {
      await pollers[0].refresh();
    });

    expect(rooms.getMessages).toHaveBeenNthCalledWith(2, {
      roomName: 'lobby',
      signal: expect.any(AbortSignal),
      since: 100,
    });
    expect(screen.getByText('one')).toBeInTheDocument();
    expect(screen.getByText('two')).toBeInTheDocument();
  });

  it('aborts room reads when the session unmounts', async () => {
    let requestSignal;
    rooms.getMessages.mockImplementation(({ signal }) => {
      requestSignal = signal;
      return new Promise((_resolve, reject) => {
        signal.addEventListener('abort', () => {
          const error = new Error('aborted');
          error.name = 'AbortError';
          reject(error);
        });
      });
    });
    rooms.getUsers.mockResolvedValue([]);

    const { unmount } = render(<RoomSession active roomName="lobby" />);
    await waitFor(() => expect(requestSignal).toBeDefined());

    unmount();

    expect(requestSignal.aborted).toBe(true);
  });
});
