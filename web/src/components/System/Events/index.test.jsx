import '@testing-library/jest-dom';
import { list } from '../../../lib/events';
import Events from './index';
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../lib/events', () => ({
  list: vi.fn(),
}));

describe('Events', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders malformed event payloads as text instead of crashing', async () => {
    list.mockResolvedValue({
      events: [
        {
          data: 'not-json',
          id: 'event-1',
          timestamp: '2026-09-03T00:00:00Z',
          type: 'SearchCreated',
        },
      ],
      totalCount: 1,
    });

    render(<Events />);

    expect(await screen.findByText('not-json')).toBeInTheDocument();
  });

  it('reports event history failures and clears the loading state', async () => {
    list.mockRejectedValue(new Error('event backend unavailable'));

    render(<Events />);

    expect(
      await screen.findByText('event backend unavailable'),
    ).toBeInTheDocument();
    expect(screen.queryByText('No events')).not.toBeInTheDocument();
  });

  it('passes applied filters to the paginated event request', async () => {
    list.mockResolvedValue({ events: [], totalCount: 0 });

    render(<Events />);

    await waitFor(() => expect(list).toHaveBeenCalled());
    fireEvent.change(screen.getByRole('textbox', { name: 'Event text filter' }), {
      target: { value: 'ambient' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'Event kind filter' }), {
      target: { value: 'search.started' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'Event topic filter' }), {
      target: { value: 'searches' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Filter events' }));

    await waitFor(() => {
      expect(list).toHaveBeenLastCalledWith({
        kind: 'search.started',
        limit: 10,
        offset: 0,
        q: 'ambient',
        topic: 'searches',
      });
    });
  });

  it('retains the previous page when a filtered refresh fails', async () => {
    list
      .mockResolvedValueOnce({
        events: [
          {
            data: { message: 'fixture event' },
            id: 'event-1',
            timestamp: '2026-09-03T00:00:00Z',
            type: 'SearchCreated',
          },
        ],
        totalCount: 1,
      })
      .mockRejectedValueOnce(new Error('filtered event history unavailable'));

    render(<Events />);
    expect(await screen.findByText(/fixture event/)).toBeInTheDocument();

    fireEvent.change(screen.getByRole('textbox', { name: 'Event text filter' }), {
      target: { value: 'ambient' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Filter events' }));

    expect(await screen.findByTestId('events-load-error')).toHaveTextContent(
      'filtered event history unavailable',
    );
    expect(screen.getByText(/fixture event/)).toBeInTheDocument();
    expect(screen.queryByText('No events')).not.toBeInTheDocument();
  });
});
