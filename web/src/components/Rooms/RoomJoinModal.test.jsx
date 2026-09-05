import '@testing-library/jest-dom';
import * as rooms from '../../lib/rooms';
import RoomJoinModal from './RoomJoinModal';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import React from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/rooms', () => ({
  getAvailable: vi.fn(),
}));

vi.mock('react-toastify', () => ({
  toast: {
    error: vi.fn(),
  },
}));

describe('RoomJoinModal', () => {
  beforeEach(() => {
    rooms.getAvailable.mockRejectedValue({
      response: { data: { message: 'Room service unavailable' } },
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('surfaces room-list failures instead of showing an empty room list', async () => {
    render(
      <RoomJoinModal
        joinRoom={vi.fn()}
        trigger={<button type="button">Open room picker</button>}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Open room picker' }));

    expect(await screen.findByTestId('room-join-load-error')).toHaveTextContent(
      'Room service unavailable',
    );
    expect(screen.getByText('Rooms unavailable')).toBeInTheDocument();
    expect(screen.queryByText('No rooms available')).not.toBeInTheDocument();
  });
});
