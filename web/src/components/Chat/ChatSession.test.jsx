import '@testing-library/jest-dom';
import * as chat from '../../lib/chat';
import ChatSession from './ChatSession';
import React from 'react';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { pollers } = vi.hoisted(() => ({ pollers: [] }));

vi.mock('../../lib/chat', () => ({
  acknowledge: vi.fn(),
  get: vi.fn(),
  send: vi.fn(),
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

describe('ChatSession', () => {
  beforeEach(() => {
    pollers.length = 0;
    vi.clearAllMocks();
    chat.acknowledge.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  it('requests cursor deltas and merges them into bounded history', async () => {
    chat.get
      .mockResolvedValueOnce({
        messages: [{ createdAtMs: 100, id: 'one', message: 'one' }],
        username: 'peer',
      })
      .mockResolvedValueOnce({
        messages: [{ createdAtMs: 200, id: 'two', message: 'two' }],
        username: 'peer',
      });

    render(
      <ChatSession
        active
        user={{ username: 'me' }}
        username="peer"
      />,
    );

    expect(await screen.findByText('one')).toBeInTheDocument();
    await act(async () => {
      await pollers[0].refresh();
    });

    expect(chat.get).toHaveBeenNthCalledWith(2, {
      since: 100,
      signal: expect.any(AbortSignal),
      username: 'peer',
    });
    expect(screen.getByText('one')).toBeInTheDocument();
    expect(screen.getByText('two')).toBeInTheDocument();
  });

  it('aborts an in-flight conversation request on unmount', async () => {
    let requestSignal;
    chat.get.mockImplementation(({ signal }) => {
      requestSignal = signal;
      return new Promise((_resolve, reject) => {
        signal.addEventListener('abort', () => {
          const error = new Error('aborted');
          error.name = 'AbortError';
          reject(error);
        });
      });
    });

    const { unmount } = render(
      <ChatSession
        active
        user={{ username: 'me' }}
        username="peer"
      />,
    );
    await waitFor(() => expect(requestSignal).toBeDefined());

    unmount();

    expect(requestSignal.aborted).toBe(true);
  });
});
