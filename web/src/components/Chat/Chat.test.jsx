import '@testing-library/jest-dom';
import * as chat from '../../lib/chat';
import Chat from './Chat';
import { cleanup, render, screen } from '@testing-library/react';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/chat', () => ({
  getAll: vi.fn(),
  remove: vi.fn(),
  send: vi.fn(),
  get: vi.fn(),
  acknowledge: vi.fn(),
}));

vi.mock('./ChatSession', () => ({ default: () => null }));

describe('Chat', () => {
  beforeEach(() => {
    chat.getAll.mockRejectedValue(new Error('Conversation service unavailable'));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    localStorage.clear();
  });

  it('surfaces saved-conversation hydration failures', async () => {
    render(
      <MemoryRouter>
        <Chat state={{ user: { username: 'local-user' } }} />
      </MemoryRouter>,
    );

    expect(await screen.findByTestId('chat-conversations-error')).toHaveTextContent(
      'Conversation service unavailable',
    );
  });
});
