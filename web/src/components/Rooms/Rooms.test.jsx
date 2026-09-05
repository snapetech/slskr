import '@testing-library/jest-dom';
import * as rooms from '../../lib/rooms';
import Rooms from './Rooms';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/rooms', () => ({
  getAvailable: vi.fn(),
  getJoined: vi.fn(),
  join: vi.fn(),
  leave: vi.fn(),
}));

vi.mock('../../lib/hubFactory', () => ({
  createRoomsHubConnection: vi.fn(() => ({
    on: vi.fn(),
    start: vi.fn().mockResolvedValue(undefined),
    stop: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock('../../lib/usePolling', () => ({
  usePolling: vi.fn(),
}));

vi.mock('./RoomCreateModal', () => ({ default: () => null }));
vi.mock('./RoomSession', () => ({ default: () => null }));

describe('Rooms', () => {
  beforeEach(() => {
    rooms.getJoined.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('does not invalidate an available-room request when an open event overlaps it', async () => {
    let resolveAvailable;
    rooms.getAvailable.mockReturnValue(
      new Promise((resolve) => {
        resolveAvailable = resolve;
      }),
    );

    const { container } = render(
      <MemoryRouter>
        <Rooms />
      </MemoryRouter>,
    );

    const dropdown = container.querySelector('.rooms-input');
    expect(dropdown).toBeTruthy();
    fireEvent.click(dropdown);
    fireEvent.click(dropdown);

    await waitFor(() => expect(rooms.getAvailable).toHaveBeenCalledTimes(1));
    resolveAvailable([{ name: 'music', userCount: 3 }]);

    fireEvent.click(dropdown);
    await waitFor(() => expect(screen.getByText('music (3 users)')).toBeInTheDocument());
  });
});
