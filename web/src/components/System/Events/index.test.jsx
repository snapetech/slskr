import '@testing-library/jest-dom';
import { list } from '../../../lib/events';
import Events from './index';
import React from 'react';
import { render, screen } from '@testing-library/react';
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
    expect(screen.getByText('No events')).toBeInTheDocument();
  });
});
